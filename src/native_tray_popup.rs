//! Dedicated Win32 popup, independent of eframe viewport focus and DPI hit testing.
#![cfg(windows)]
use std::{sync::{Arc,atomic::{AtomicU8,AtomicU64,Ordering}},thread,time::Duration};
use windows_sys::Win32::{Foundation::{HWND,LPARAM,LRESULT,WPARAM,RECT},UI::WindowsAndMessaging::*,Graphics::Gdi::*};
use chrono::{Datelike,Local,Duration as ChronoDuration};
// Win32 mouse capture functions are provided by user32.dll.
// Declare explicitly to avoid missing imports in windows-sys 0.59.
#[link(name="user32")]
extern "system" {
    fn SetCapture(hwnd: HWND) -> HWND;
    fn ReleaseCapture() -> i32;
}

use crate::{g2j,j2g,mlen,MONTHS,pd};
const BG:u32=0x002F1808; // RGB(8,24,47) as COLORREF
const BTN_PREV:usize=101;
const BTN_NEXT:usize=102;
fn wide(s:&str)->Vec<u16>{s.encode_utf16().chain(Some(0)).collect()}
struct State{y:i32,m:u32,header:HWND,brush:HBRUSH,font:HFONT,note_request:Arc<AtomicU64>}
unsafe fn update_labels(st:&mut State){
 SetWindowTextW(st.header,wide(&format!("{} {}",MONTHS[(st.m-1) as usize],pd(st.y))).as_ptr());
}
// Read the same persisted holiday flags used by the main calendar; never invent official holidays.
fn saved_holidays()->std::collections::HashSet<String>{
 let mut found=std::collections::HashSet::new();
 if let Ok(data)=std::fs::read_to_string(crate::path()){
  if let Ok(json)=serde_json::from_str::<serde_json::Value>(&data){
   if let Some(days)=json.get("days").and_then(|v|v.as_object()){
    for (date,entry) in days {if entry.get("holiday").is_some_and(|v|!v.is_null()) {found.insert(date.clone());}}
   }
  }
 }
 found
}
unsafe fn paint_calendar(hwnd:HWND,st:&State,dc:HDC){
 let mut client=std::mem::zeroed();GetClientRect(hwnd,&mut client);FillRect(dc,&client,st.brush);
 let old=SelectObject(dc,st.font as _);SetBkMode(dc,TRANSPARENT as i32);
 let holidays=saved_holidays();
 let (gy,gm,gd)=j2g(st.y,st.m,1);
 let first=chrono::NaiveDate::from_ymd_opt(gy,gm,gd).unwrap();
 let offset=((first.weekday().num_days_from_monday()+2)%7) as usize;
 let today=Local::now();let actual=g2j(today.year(),today.month(),today.day());
 // Columns are physically right-to-left: Saturday at the far right, Friday at the far left.
 const LEFT:i32=11;const CELL_W:i32=43;const TOP:i32=57;const ROW_H:i32=32;
 for weekday in 0..7usize{
  let x=LEFT+(6-weekday as i32)*CELL_W;
  let mut rect=RECT{left:x,top:TOP,right:x+CELL_W,bottom:TOP+29};
  SetTextColor(dc,if weekday==6{0x005a68ff}else{0x00dcecff});
  const SHORT_WEEK:[&str;7]=["ش","ی","د","س","چ","پ","ج"];
  let mut t=wide(SHORT_WEEK[weekday]);
  DrawTextW(dc,t.as_mut_ptr(),-1,&mut rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_RTLREADING);
 }
 let total=mlen(st.y,st.m) as usize;
 for cell in 0..42usize{
  let weekday=cell%7;
  let x=LEFT+(6-weekday as i32)*CELL_W;
  let y=TOP+34+(cell/7) as i32*ROW_H;
  let index=cell as i32-offset as i32;
  let date=first+ChronoDuration::days(index as i64);
  let (jy,jm,jd)=g2j(date.year(),date.month(),date.day());
  let adjacent=index<0 || index>=total as i32;
  let is_today=(jy,jm,jd)==actual;
  let red=weekday==6 || holidays.contains(&crate::key(jy,jm,jd)) || crate::official_events(jy,jm,jd).iter().any(|event|event.holiday);
  if is_today{
   let hl=CreateSolidBrush(0x009f571b);
   let rect=RECT{left:x+3,top:y+2,right:x+CELL_W-3,bottom:y+ROW_H-3};
   FillRect(dc,&rect,hl);DeleteObject(hl as _);
  }
  let color=if red{0x005a68ff}else if adjacent{0x008a9ba8}else{0x00ffffff};
  SetTextColor(dc,color);
  let mut rect=RECT{left:x,top:y,right:x+CELL_W,bottom:y+ROW_H};
  let mut text=wide(&pd(jd));
  DrawTextW(dc,text.as_mut_ptr(),-1,&mut rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_RTLREADING);
 }
 // Gregorian date footer for the real current day, matching the Sun Calendar layout.
 const GREG_MONTHS:[&str;12]=["ژانویه","فوریه","مارس","آوریل","مه","ژوئن","ژوئیه","اوت","سپتامبر","اکتبر","نوامبر","دسامبر"];
 let footer=format!("{} {} {}",today.day(),GREG_MONTHS[(today.month()-1) as usize],today.year());
 let mut footer_text=wide(&footer);
 let mut footer_rect=RECT{left:12,top:299,right:width_or_client_right(hwnd)-12,bottom:329};
 SetTextColor(dc,0x00e0e8f3);
 let pen=CreatePen(PS_SOLID,1,0x00432e25);
 let old_pen=SelectObject(dc,pen as _);
 MoveToEx(dc,12,295,std::ptr::null_mut());LineTo(dc,footer_rect.right,295);
 SelectObject(dc,old_pen);DeleteObject(pen as _);
 DrawTextW(dc,footer_text.as_mut_ptr(),-1,&mut footer_rect,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_RTLREADING);
 SelectObject(dc,old);
}
fn width_or_client_right(hwnd:HWND)->i32{unsafe{let mut rect:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rect);rect.right}}
unsafe extern "system" fn wndproc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM)->LRESULT{
 match msg{
  WM_LBUTTONDBLCLK=>{
   let st=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State;
   if !st.is_null(){
    let x=(lp as u32 & 0xffff) as i16 as i32;
    let y=((lp as u32 >> 16)&0xffff) as i16 as i32;
    const LEFT:i32=11;const CELL_W:i32=43;const TOP:i32=91;const ROW_H:i32=32;
    if y>=TOP && y<TOP+6*ROW_H && x>=LEFT && x<LEFT+7*CELL_W {
     let col=(x-LEFT)/CELL_W;
     let row=(y-TOP)/ROW_H;
     let weekday=6-col;
     let (gy,gm,gd)=j2g((*st).y,(*st).m,1);
     if let Some(first)=chrono::NaiveDate::from_ymd_opt(gy,gm,gd){
      let offset=((first.weekday().num_days_from_monday()+2)%7) as i32;
      let index=row*7+weekday-offset;
      let date=first+ChronoDuration::days(index as i64);
      let (jy,jm,jd)=g2j(date.year(),date.month(),date.day());
      // Encode selected Jalali date; zero means no request.
      (*st).note_request.store((jy as u64)*10000+(jm as u64)*100+jd as u64,Ordering::SeqCst);
      ShowWindow(hwnd,SW_HIDE);
     }
    }
   }
   0
  }
  WM_COMMAND=>{
   let st=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State;
   if !st.is_null(){match wp&0xffff{
    BTN_PREV=>{if (*st).m==1{(*st).m=12;(*st).y-=1}else{(*st).m-=1}update_labels(&mut *st);InvalidateRect(hwnd,std::ptr::null(),1);},
    BTN_NEXT=>{if (*st).m==12{(*st).m=1;(*st).y+=1}else{(*st).m+=1}update_labels(&mut *st);InvalidateRect(hwnd,std::ptr::null(),1);},
    _=>{}
   }}
   0
  }
  WM_PAINT=>{let st=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State;let mut ps:PAINTSTRUCT=std::mem::zeroed();let dc=BeginPaint(hwnd,&mut ps);if !st.is_null(){paint_calendar(hwnd,&*st,dc);}EndPaint(hwnd,&ps);0}
  WM_CTLCOLORSTATIC=>{
   let st=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State;
   if !st.is_null(){SetTextColor(wp as HDC,0x00ffffff);SetBkMode(wp as HDC,TRANSPARENT as i32);return (*st).brush as LRESULT;}
   DefWindowProcW(hwnd,msg,wp,lp)
  }
  WM_ERASEBKGND=>{let st=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut State;
   if !st.is_null(){let mut r=std::mem::zeroed();GetClientRect(hwnd,&mut r);FillRect(wp as HDC,&r,(*st).brush);return 1;}0}
  WM_ACTIVATE=>{if (wp & 0xffff) == WA_INACTIVE as usize { ShowWindow(hwnd,SW_HIDE); } 0}
  WM_CLOSE=>{ShowWindow(hwnd,SW_HIDE);0}
  _=>DefWindowProcW(hwnd,msg,wp,lp)
 }
}
unsafe fn label(parent:HWND,text:&str,x:i32,y:i32,w:i32,h:i32,font:HFONT)->HWND{
 let hwnd=CreateWindowExW(0,wide("STATIC").as_ptr(),wide(text).as_ptr(),WS_CHILD|WS_VISIBLE|1u32 /* SS_CENTER (Win32 STATIC style) */,
 x,y,w,h,parent,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null());
 SendMessageW(hwnd,WM_SETFONT,font as usize,1);hwnd
}
// Clickable STATIC labels keep the navy header seamless (no white Win32 button borders).
// SS_NOTIFY sends WM_COMMAND with the same IDs as the original buttons.
unsafe fn nav_arrow(parent:HWND,text:&str,id:usize,x:i32,y:i32,font:HFONT)->HWND{
 let hwnd=CreateWindowExW(0,wide("STATIC").as_ptr(),wide(text).as_ptr(),WS_CHILD|WS_VISIBLE|1u32|0x0100u32, // SS_CENTER | SS_NOTIFY
 x,y,24,32,parent,id as *mut _,std::ptr::null_mut(),std::ptr::null());
 SendMessageW(hwnd,WM_SETFONT,font as usize,1);hwnd
}

fn run(commands:Arc<AtomicU8>,note_request:Arc<AtomicU64>){unsafe{
 let class=wide("SushiCalendarNativeTrayPopup_v2566");
 let instance=windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
 let brush=CreateSolidBrush(BG);
 let wc=WNDCLASSW{style:CS_HREDRAW|CS_VREDRAW|CS_DBLCLKS,lpfnWndProc:Some(wndproc),hInstance:instance,hbrBackground:brush,lpszClassName:class.as_ptr(),..std::mem::zeroed()};
 RegisterClassW(&wc);
 let width=326;let height=340;
 // Initial coordinates are provisional. Windows 10 may not have initialized the
 // taskbar/work area when this background thread starts. Recalculate on every show.
 let x=(GetSystemMetrics(SM_CXSCREEN)-width-15).max(0);
 let y=(GetSystemMetrics(SM_CYSCREEN)-height-65).max(0);
 let hwnd=CreateWindowExW(WS_EX_TOPMOST|WS_EX_TOOLWINDOW,class.as_ptr(),wide("Sushi Mini Calendar").as_ptr(),WS_POPUP|WS_CLIPCHILDREN,
 x,y,width,height,std::ptr::null_mut(),std::ptr::null_mut(),instance,std::ptr::null());
 if hwnd.is_null(){crate::diagnostic_log("NATIVE_POPUP create failed");return;}
 let font=CreateFontW(-17,0,0,0,FW_NORMAL as i32,0,0,0,DEFAULT_CHARSET as u32,OUT_DEFAULT_PRECIS as u32,CLIP_DEFAULT_PRECIS as u32,CLEARTYPE_QUALITY as u32,DEFAULT_PITCH as u32,wide("Segoe UI").as_ptr());
 let (yy,mm,_)=crate::ty_now();
 // Centered month label, small borderless arrows drawn close to the title.
 let header=label(hwnd,"",99,14,128,32,font);
 nav_arrow(hwnd,"‹",BTN_PREV,65,14,font);
 nav_arrow(hwnd,"›",BTN_NEXT,237,14,font);
 let mut state=Box::new(State{y:yy,m:mm,header,brush,font,note_request});
 update_labels(&mut state);
 SetWindowLongPtrW(hwnd,GWLP_USERDATA,(&mut *state as *mut State) as isize);
 let region=CreateRoundRectRgn(0,0,width,height,22,22);
 SetWindowRgn(hwnd,region,1);
 crate::diagnostic_log("NATIVE_POPUP ready");
 loop{
  let cmd=commands.swap(0,Ordering::SeqCst);
  if cmd==1{
   if IsWindowVisible(hwnd)==0{
    let (yy,mm,_)=crate::ty_now();state.y=yy;state.m=mm;update_labels(&mut state);InvalidateRect(hwnd,std::ptr::null(),1);
    // Query the CURRENT desktop work area on each click, not at process startup.
    // SPI_GETWORKAREA excludes the primary taskbar on Windows 10 and Windows 11.
    let mut work:RECT=std::mem::zeroed();
    let (px,py)=if SystemParametersInfoW(SPI_GETWORKAREA,0,&mut work as *mut _ as *mut _,0)!=0 {
     ((work.right-width-10).max(work.left),(work.bottom-height-8).max(work.top))
    } else {
     ((GetSystemMetrics(SM_CXSCREEN)-width-15).max(0),
      (GetSystemMetrics(SM_CYSCREEN)-height-65).max(0))
    };
    SetWindowPos(hwnd,HWND_TOPMOST,px,py,width,height,SWP_SHOWWINDOW);
    ShowWindow(hwnd,SW_SHOW);SetForegroundWindow(hwnd);
    crate::diagnostic_log("NATIVE_POPUP shown");
   }else{ShowWindow(hwnd,SW_HIDE);crate::diagnostic_log("NATIVE_POPUP hidden");}
  }else if cmd==2{ShowWindow(hwnd,SW_HIDE);}
  let mut msg:MSG=std::mem::zeroed();
  while PeekMessageW(&mut msg,std::ptr::null_mut(),0,0,PM_REMOVE)!=0{
   if msg.message==WM_QUIT{return;}
   TranslateMessage(&msg);DispatchMessageW(&msg);
  }
  thread::sleep(Duration::from_millis(20));
 }
}}
pub fn start(note_request:Arc<AtomicU64>)->Arc<AtomicU8>{let c=Arc::new(AtomicU8::new(0));let thread_cmd=c.clone();thread::spawn(move||run(thread_cmd,note_request));c}


// Sushi desktop widget: ten selectable themes, saved position and opacity.
// All widget Win32 calls stay on its own UI thread.
#[derive(serde::Serialize,serde::Deserialize,Clone)]
struct WidgetPrefs { x:i32,y:i32,theme:usize,opacity:u8 }
impl Default for WidgetPrefs{
 fn default()->Self{Self{x:45,y:90,theme:0,opacity:204}}
}
fn widget_prefs_path()->std::path::PathBuf{
 dirs::config_dir().unwrap_or_else(||std::path::PathBuf::from("."))
  .join("SushiCalendar").join("desktop_widget.json")
}
fn load_widget_prefs()->WidgetPrefs{
 std::fs::read_to_string(widget_prefs_path()).ok()
  .and_then(|s|serde_json::from_str(&s).ok()).unwrap_or_default()
}
fn save_widget_prefs(p:&WidgetPrefs){
 let path=widget_prefs_path();
 if let Some(parent)=path.parent(){let _=std::fs::create_dir_all(parent);}
 if let Ok(json)=serde_json::to_vec_pretty(p){let _=std::fs::write(path,json);}
}
const WIDGET_THEMES:[(&str,u32,u32);10]=[
 ("۱. مشکی کلاسیک",0x00151515,0),
 ("۲. دودی شفاف",0x00232120,0),
 ("۳. خاکستری گرم",0x00342a30,0),
 ("۴. شیشه ای",0x00463832,0),
 ("۵. آبی درخشان",0x00352109,0x00ffae48),
 ("۶. نارنجی درخشان",0x001b2a44,0x004da1ff),
 ("۷. خاکستری گرد",0x003c3436,0x00dddddd),
 ("۸. سایه ملایم",0x0019161a,0x009e9e9e),
 ("۹. شیشه روشن",0x00554c4b,0x00ffffff),
 ("۱۰. آبی مات",0x003b251d,0x009edaff),
];
struct WidgetState { request:Arc<AtomicU64>,prefs:WidgetPrefs,brush:HBRUSH,drag:bool,drag_x:i32,drag_y:i32,origin_x:i32,origin_y:i32 }
unsafe fn apply_widget_style(hwnd:HWND,p:&WidgetPrefs){
 // WS_EX_LAYERED allows per-window opacity, without changing note hit testing.
 let ex=GetWindowLongPtrW(hwnd,GWL_EXSTYLE);
 SetWindowLongPtrW(hwnd,GWL_EXSTYLE,ex | WS_EX_LAYERED as isize);
 SetLayeredWindowAttributes(hwnd,0,p.opacity, LWA_ALPHA);
 InvalidateRect(hwnd,std::ptr::null(),1);
}
unsafe fn widget_menu(hwnd:HWND,st:&mut WidgetState){
 let menu=CreatePopupMenu();
 for (i,(name,_,_)) in WIDGET_THEMES.iter().enumerate(){
  let label=wide(name);
  AppendMenuW(menu,MF_STRING | if st.prefs.theme==i{MF_CHECKED}else{0},(1000+i) as usize,label.as_ptr());
 }
 AppendMenuW(menu,MF_SEPARATOR,0,std::ptr::null());
 for (i,opacity) in [255u8,230,204,178,153,128,102].iter().enumerate(){
  let label=wide(&format!("شفافیت: {}٪",(*opacity as u32)*100/255));
  AppendMenuW(menu,MF_STRING | if st.prefs.opacity==*opacity{MF_CHECKED}else{0},(1100+i) as usize,label.as_ptr());
 }
 let mut pt:windows_sys::Win32::Foundation::POINT=std::mem::zeroed();
 GetCursorPos(&mut pt);
 SetForegroundWindow(hwnd);
 let selected=TrackPopupMenu(menu,TPM_RETURNCMD|TPM_RIGHTBUTTON,pt.x,pt.y,0,hwnd,std::ptr::null());
 if (1000..1010).contains(&selected){st.prefs.theme=(selected-1000) as usize;}
 else if (1100..1107).contains(&selected){st.prefs.opacity=[255,230,204,178,153,128,102][(selected-1100) as usize];}
 save_widget_prefs(&st.prefs);
 apply_widget_style(hwnd,&st.prefs);
 DestroyMenu(menu);
}
unsafe extern "system" fn widget_proc(hwnd:HWND,msg:u32,wp:WPARAM,lp:LPARAM)->LRESULT{
 let p=GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *mut WidgetState;
 match msg{
  WM_LBUTTONDBLCLK=>{
   if !p.is_null(){let(y,m,d)=crate::ty_now();(*p).request.store((y as u64)*10000+(m as u64)*100+d as u64,Ordering::SeqCst);}
   0
  },
  WM_LBUTTONDOWN=>{
   if !p.is_null(){
    // Capture cursor position in SCREEN coordinates. Client coordinates
    // change as the widget moves, causing jumpy/slow feedback loops.
    let mut cursor:windows_sys::Win32::Foundation::POINT=std::mem::zeroed();
    GetCursorPos(&mut cursor);
    (*p).drag=true;(*p).drag_x=cursor.x;(*p).drag_y=cursor.y;
    let mut rect:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut rect);
    (*p).origin_x=rect.left;(*p).origin_y=rect.top;
    SetCapture(hwnd);
   } 0
  },
  WM_MOUSEMOVE=>{
   if !p.is_null()&&(*p).drag{
    let mut cursor:windows_sys::Win32::Foundation::POINT=std::mem::zeroed();
    if GetCursorPos(&mut cursor)!=0 {
     let nx=(*p).origin_x+cursor.x-(*p).drag_x;
     let ny=(*p).origin_y+cursor.y-(*p).drag_y;
     SetWindowPos(hwnd,std::ptr::null_mut(),nx,ny,0,0,SWP_NOSIZE|SWP_NOZORDER|SWP_NOACTIVATE);
    }
   }0
  },
  WM_LBUTTONUP=>{
   if !p.is_null()&&(*p).drag{
    (*p).drag=false;ReleaseCapture();
    let mut rect:RECT=std::mem::zeroed();GetWindowRect(hwnd,&mut rect);
    (*p).prefs.x=rect.left;(*p).prefs.y=rect.top;save_widget_prefs(&(*p).prefs);
   }0
  },
  WM_RBUTTONUP=>{if !p.is_null(){widget_menu(hwnd,&mut *p);}0},
  WM_PAINT=>{
   let mut ps:PAINTSTRUCT=std::mem::zeroed();let dc=BeginPaint(hwnd,&mut ps);
   if !p.is_null(){
    let theme=(*p).prefs.theme.min(9);
    let (_,bg,border)=WIDGET_THEMES[theme];
    let mut rc:RECT=std::mem::zeroed();GetClientRect(hwnd,&mut rc);
    let brush=CreateSolidBrush(bg);FillRect(dc,&rc,brush);DeleteObject(brush as _);
    if border!=0{
     let pen=CreatePen(PS_SOLID,2,border);let old=SelectObject(dc,pen as _);
     let nullbrush=GetStockObject(NULL_BRUSH as i32);let oldbrush=SelectObject(dc,nullbrush);
     RoundRect(dc,1,1,199,209,30,30);
     SelectObject(dc,oldbrush);SelectObject(dc,old);DeleteObject(pen as _);
    }
    SetBkMode(dc,TRANSPARENT as i32);SetTextColor(dc,0x00ffffff);
    let(y,m,d)=crate::ty_now();
    // IranNastaliq when installed; Windows GDI falls back when absent.
    let font=CreateFontW(-26,0,0,0,FW_NORMAL as i32,0,0,0,DEFAULT_CHARSET as u32,OUT_DEFAULT_PRECIS as u32,CLIP_DEFAULT_PRECIS as u32,CLEARTYPE_QUALITY as u32,DEFAULT_PITCH as u32,wide("IranNastaliq").as_ptr());
    let old=SelectObject(dc,font as _);
    let mut header=wide(crate::weekday(y,m,d));let mut head=RECT{left:8,top:6,right:192,bottom:55};
    DrawTextW(dc,header.as_mut_ptr(),-1,&mut head,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_RTLREADING);
    let mut footer=wide(&format!("{} {}",MONTHS[(m-1)as usize],pd(y)));let mut foot=RECT{left:8,top:155,right:192,bottom:205};
    DrawTextW(dc,footer.as_mut_ptr(),-1,&mut foot,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_RTLREADING);
    SelectObject(dc,old);DeleteObject(font as _);
    let big=CreateFontW(-82,0,0,0,FW_BOLD as i32,0,0,0,DEFAULT_CHARSET as u32,OUT_DEFAULT_PRECIS as u32,CLIP_DEFAULT_PRECIS as u32,CLEARTYPE_QUALITY as u32,DEFAULT_PITCH as u32,wide("Tahoma").as_ptr());
    let old=SelectObject(dc,big as _);let mut day=wide(&pd(d));let mut dr=RECT{left:8,top:48,right:192,bottom:156};
    DrawTextW(dc,day.as_mut_ptr(),-1,&mut dr,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_RTLREADING);
    SelectObject(dc,old);DeleteObject(big as _);
   }
   EndPaint(hwnd,&ps);0
  },
  _=>DefWindowProcW(hwnd,msg,wp,lp)
 }
}
pub fn start_desktop_widget(request:Arc<AtomicU64>){
 thread::spawn(move||unsafe{
  let instance=windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());
  let class=wide("SushiDesktopWidget2593");
  let wc=WNDCLASSW{style:CS_HREDRAW|CS_VREDRAW|CS_DBLCLKS,lpfnWndProc:Some(widget_proc),hInstance:instance,lpszClassName:class.as_ptr(),..std::mem::zeroed()};
  RegisterClassW(&wc);
  let mut prefs=load_widget_prefs();
  prefs.theme=prefs.theme.min(9);
  prefs.opacity=prefs.opacity.max(75);
  // Clamp saved position to the current desktop after display changes.
  let sw=GetSystemMetrics(SM_CXSCREEN);let sh=GetSystemMetrics(SM_CYSCREEN);
  prefs.x=prefs.x.clamp(0,(sw-200).max(0));prefs.y=prefs.y.clamp(0,(sh-210).max(0));
  let hwnd=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_LAYERED,class.as_ptr(),wide("Sushi Desktop Widget").as_ptr(),WS_POPUP|WS_VISIBLE,prefs.x,prefs.y,200,210,std::ptr::null_mut(),std::ptr::null_mut(),instance,std::ptr::null());
  if hwnd.is_null(){crate::diagnostic_log("DESKTOP_WIDGET create failed");return;}
  let mut state=Box::new(WidgetState{request,prefs,brush:std::ptr::null_mut(),drag:false,drag_x:0,drag_y:0,origin_x:0,origin_y:0});
  SetWindowLongPtrW(hwnd,GWLP_USERDATA,(&mut *state as *mut WidgetState) as isize);
  SetWindowRgn(hwnd,CreateRoundRectRgn(0,0,200,210,30,30),1);
  apply_widget_style(hwnd,&state.prefs);ShowWindow(hwnd,SW_SHOW);
  let mut last=crate::ty_now();loop{
   let mut msg:MSG=std::mem::zeroed();while PeekMessageW(&mut msg,std::ptr::null_mut(),0,0,PM_REMOVE)!=0{if msg.message==WM_QUIT{return;}TranslateMessage(&msg);DispatchMessageW(&msg);}
   let now=crate::ty_now();if now!=last{last=now;InvalidateRect(hwnd,std::ptr::null(),1);}
   thread::sleep(Duration::from_millis(40));
  }
 });
}
