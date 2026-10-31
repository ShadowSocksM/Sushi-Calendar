#![cfg_attr(not(debug_assertions), windows_subsystem="windows")]
#[cfg(windows)] mod native_tray_popup;
use chrono::{Datelike,Local,NaiveDate,Timelike,Weekday};
use eframe::egui::{self,Color32,RichText,FontData,FontDefinitions,FontFamily};
use serde::{Serialize,Deserialize};
use std::{collections::BTreeMap,fs,path::PathBuf,sync::{Arc,atomic::{AtomicU8,AtomicU64,Ordering}}};
use tray_icon::{TrayIcon,TrayIconBuilder,Icon,TrayIconEvent,MouseButton,menu::{Menu,MenuItem,MenuEvent,MenuId}};

#[cfg(windows)]
static SECOND_LAUNCH:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
#[cfg(windows)]
static STARTUP_HIDDEN:std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
#[cfg(windows)]
const SINGLE_EVENT:&str="Local\\SushiCalendar_OpenExisting_v251";
#[cfg(windows)]
fn single_instance_guard(startup_launch:bool)->Option<(windows_sys::Win32::Foundation::HANDLE,windows_sys::Win32::Foundation::HANDLE)>{
 use windows_sys::Win32::System::Threading::{CreateEventW,CreateMutexW,SetEvent};
 use windows_sys::Win32::Foundation::{GetLastError,CloseHandle,ERROR_ALREADY_EXISTS};
 unsafe{
  let ev=CreateEventW(std::ptr::null(),0,0,wstr(SINGLE_EVENT).as_ptr());
  if ev.is_null(){return None;}
  let mutex=CreateMutexW(std::ptr::null(),0,wstr("Local\\SushiCalendar_SingleInstance_v251").as_ptr());
  if mutex.is_null(){CloseHandle(ev);return None;}
  if GetLastError()==ERROR_ALREADY_EXISTS{
   if !startup_launch { SetEvent(ev); } CloseHandle(mutex);CloseHandle(ev);return None;
  }
  Some((mutex,ev))
 }
}
const MONTHS:[&str;12]=["فروردین","اردیبهشت","خرداد","تیر","مرداد","شهریور","مهر","آبان","آذر","دی","بهمن","اسفند"];
const WEEK:[&str;7]=["شنبه","یکشنبه","دوشنبه","سه‌شنبه","چهارشنبه","پنجشنبه","جمعه"];
const GMON:[&str;12]=["January","February","March","April","May","June","July","August","September","October","November","December"];
const NAVY:Color32=Color32::from_rgb(8,24,47);
const PANEL:Color32=Color32::from_rgb(12,35,66);
const CARD:Color32=Color32::from_rgb(18,48,84);
const BLUE:Color32=Color32::from_rgb(36,112,240);

#[cfg(windows)]
fn restore_native_window(){
 use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW,ShowWindow,SetForegroundWindow,SW_SHOW,SW_RESTORE};
 let title:Vec<u16>="Sushi Calendar v2.5.95\0".encode_utf16().collect();
 unsafe{
  let hwnd=FindWindowW(std::ptr::null(),title.as_ptr());
  if !hwnd.is_null(){ShowWindow(hwnd,SW_SHOW);ShowWindow(hwnd,SW_RESTORE);SetForegroundWindow(hwnd);}
 }
}
#[cfg(windows)]
fn hide_main_native(){
 use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW,ShowWindow,SW_HIDE};
 let title=wstr("Sushi Calendar v2.5.95");
 unsafe {let hwnd=FindWindowW(std::ptr::null(),title.as_ptr());if !hwnd.is_null(){ShowWindow(hwnd,SW_HIDE);}}
}
#[cfg(not(windows))]
fn restore_native_window(){}
#[cfg(windows)]
fn wake_native_window_for_mini(){
 use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW,ShowWindow,SW_SHOWNOACTIVATE};
 let title:Vec<u16>="Sushi Calendar v2.5.95\0".encode_utf16().collect();
 unsafe {let hwnd=FindWindowW(std::ptr::null(),title.as_ptr());if !hwnd.is_null(){ShowWindow(hwnd,SW_SHOWNOACTIVATE);}}
}
#[cfg(not(windows))]
fn wake_native_window_for_mini(){}


fn diagnostic_log(message:&str){
 use std::io::Write;
 let file=std::env::current_exe().ok().and_then(|p|p.parent().map(|d|d.join("SushiCalendar_Tray_Debug.log")));
 if let Some(path)=file{if let Ok(mut f)=std::fs::OpenOptions::new().create(true).append(true).open(path){
  let _=writeln!(f,"{} {}",chrono::Local::now().format("%H:%M:%S%.3f"),message);
 }}
}
fn ty_now()->(i32,u32,u32){let n=Local::now();g2j(n.year(),n.month(),n.day())}
fn pd<T:ToString>(v:T)->String{
 v.to_string().chars().map(|c|match c{'0'=>'۰','1'=>'۱','2'=>'۲','3'=>'۳','4'=>'۴','5'=>'۵','6'=>'۶','7'=>'۷','8'=>'۸','9'=>'۹',x=>x}).collect()
}
fn fa(t:&str)->String{
 if !t.chars().any(|c|matches!(c as u32,0x0600..=0x06ff|0x0750..=0x077f|0x08a0..=0x08ff)){return t.into()}
 let shaped=arabic_reshaper::arabic_reshape(t);let bidi=unicode_bidi::BidiInfo::new(&shaped,None);let mut o=String::new();
 for p in &bidi.paragraphs{o.push_str(&bidi.reorder_line(p,p.range.clone()));}o
}
fn fonts(ctx:&egui::Context){
 for p in [r"C:\Windows\Fonts\tahoma.ttf",r"C:\Windows\Fonts\segoeui.ttf",r"C:\Windows\Fonts\arial.ttf"]{
  if let Ok(b)=fs::read(p){let mut f=FontDefinitions::default();f.font_data.insert("fa".into(),FontData::from_owned(b).into());
   for fam in [FontFamily::Proportional,FontFamily::Monospace]{if let Some(v)=f.families.get_mut(&fam){v.insert(0,"fa".into())}}
   ctx.set_fonts(f);break}
 }
}
fn tray_day_icon(day:u32)->Option<Icon>{
 // 16x16 Windows tray icon: render bold Persian digits directly at native size.
 // Persian zero is drawn as a proper oval, rather than the tiny dot in many fonts.
 let font_bytes = fs::read(r"C:\Windows\Fonts\tahomabd.ttf")
  .or_else(|_| fs::read(r"C:\Windows\Fonts\seguisb.ttf"))
  .or_else(|_| fs::read(r"C:\Windows\Fonts\tahoma.ttf"))
  .ok()?;
 let font=fontdue::Font::from_bytes(font_bytes,fontdue::FontSettings::default()).ok()?;
 let persian=pd(day);
 let digits:Vec<char>=persian.chars().collect();
 let size=14.0f32; // Fit full-height Persian digits into the native 16px tray icon.
 let mut glyphs:Vec<(usize,usize,Vec<u8>,bool)>=Vec::new();
 for c in digits{
  if c=='۰'{
   // Compact filled Persian zero: solid diamond, positioned below the other digits.
   // 4x5 alpha bitmap, no hollow center, with softened corner pixels.
   let (w,h)=(4usize,5usize);
   let bitmap:Vec<u8>=vec![
       0, 100, 100,   0,
     110, 255, 255, 110,
     255, 255, 255, 255,
     110, 255, 255, 110,
       0, 100, 100,   0,
   ];
   glyphs.push((w,h,bitmap,true));
  }else{
   let (m,bm)=font.rasterize(c,size);
   glyphs.push((m.width,m.height,bm,false));
  }
 }
 let gap=1usize;
 let total_w=glyphs.iter().map(|(w,_,_,_)|*w).sum::<usize>()+gap*glyphs.len().saturating_sub(1);
 let max_h=glyphs.iter().map(|(_,h,_,_)|*h).max().unwrap_or(1);
 let mut rgba=vec![0u8;16*16*4];
 let mut xoff=16usize.saturating_sub(total_w)/2;
 let yoff=16usize.saturating_sub(max_h)/2;
 for (w,h,bm,is_zero) in glyphs{
  for gy in 0..h{for gx in 0..w{
   // Align the filled zero near the lower baseline of the other digits.
   // Unlike the old fixed +5 offset this cannot push the zero out of the icon.
   let zero_offset=max_h.saturating_sub(h).saturating_sub(1);
   let x=xoff+gx;let y=yoff+gy+if is_zero { zero_offset } else { 0 };
   if x<16 && y<16{
    let i=(y*16+x)*4;
    rgba[i]=255;rgba[i+1]=255;rgba[i+2]=255;
    rgba[i+3]=rgba[i+3].max(bm[gy*w+gx]);
   }
  }}
  xoff+=w+gap;
 }
 Icon::from_rgba(rgba,16,16).ok()
}

fn tray()->Option<(TrayIcon,MenuId,MenuId,MenuId,MenuId)>{
 let n=Local::now();let(y,m,d)=g2j(n.year(),n.month(),n.day());
 let icon=tray_day_icon(d)?;
 let menu=Menu::new();
 let open=MenuItem::new("Open Sushi Calendar",true,None);
 let today=MenuItem::new("Today",true,None);
 let note=MenuItem::new("New Note",true,None);
 let exit=MenuItem::new("Exit",true,None);
 let ids=(open.id().clone(),today.id().clone(),note.id().clone(),exit.id().clone());
 let _=menu.append_items(&[&open,&today,&note,&exit]);
 let wd=weekday(y,m,d);
 // Native tooltip: force an RTL paragraph; do not pass through fa()/reshaping.
 let tip=format!("\u{202B}{} {} {} {}\u{202C}",wd,pd(d),MONTHS[(m-1)as usize],pd(y));
 let tr=TrayIconBuilder::new().with_tooltip(tip).with_icon(icon).with_menu(Box::new(menu)).with_menu_on_left_click(false).build().ok()?;
 Some((tr,ids.0,ids.1,ids.2,ids.3))
}
#[derive(Clone,Serialize,Deserialize,Default)]struct Reminder{text:String,time:String,repeat:String,done:bool,last:String}
#[derive(Clone,Serialize,Deserialize,Default)]struct Day{holiday:Option<String>,notes:Vec<String>,reminders:Vec<Reminder>}
#[derive(Serialize,Deserialize)]struct Store{days:BTreeMap<String,Day>,startup:bool,notify:bool}
impl Default for Store{fn default()->Self{Self{days:BTreeMap::new(),startup:true,notify:true}}}
fn legacy_path()->PathBuf{let mut p=dirs::data_dir().unwrap_or(".".into());p.push("MRCalendar");p.push("data.json");p}
fn path()->PathBuf{
 let mut p=std::env::current_exe().ok().and_then(|x|x.parent().map(|p|p.to_path_buf())).unwrap_or_else(||dirs::data_dir().unwrap_or(".".into()));
 p.push("SushiCalendar_Config.json");p
}
fn load()->Store{
 let p=path();
 if let Ok(x)=fs::read_to_string(&p){if let Ok(v)=serde_json::from_str(&x){return v}}
 let old=legacy_path();
 if let Ok(x)=fs::read_to_string(&old){if let Ok(v)=serde_json::from_str::<Store>(&x){save(&v);return v}}
 Store::default()
}
fn save(s:&Store){if let Ok(x)=serde_json::to_string_pretty(s){let p=path();if let Some(d)=p.parent(){let _=fs::create_dir_all(d);}let tmp=p.with_extension("json.tmp");if fs::write(&tmp,&x).is_ok(){let _=fs::remove_file(&p);let _=fs::rename(&tmp,&p);}}}
fn reload_config()->Option<Store>{fs::read_to_string(path()).ok().and_then(|x|serde_json::from_str(&x).ok())}
fn key(y:i32,m:u32,d:u32)->String{format!("{y:04}/{m:02}/{d:02}")}
// v2.5.80: recurring Solar Hijri events, plus optional vetted per-year overrides.
// Lunar-based dates MUST be supplied in an authoritative year-specific JSON dataset.
#[derive(Clone,Serialize,Deserialize,Default)]
struct CalendarEvent { title:String, #[serde(default)] holiday:bool }
fn official_events(y:i32,m:u32,d:u32)->Vec<CalendarEvent>{
 if !(1400..=1453).contains(&y){return Vec::new()}
 let mut result=Vec::new();
 // Fixed-date occasions only. These are NOT a complete official holiday calendar.
 let fixed:&[(u32,u32,&str,bool)]=&[
  (1,1,"نوروز",true),(1,2,"عید نوروز",true),(1,3,"عید نوروز",true),(1,4,"عید نوروز",true),
  (1,12,"روز جمهوری اسلامی",true),(1,13,"روز طبیعت",true),
  (2,12,"روز معلم",false),(2,25,"روز پاسداشت زبان فارسی",false),
  (3,14,"رحلت امام خمینی",true),(3,15,"قیام پانزده خرداد",true),
  (4,1,"روز اصناف",false),(4,7,"روز قوه قضائیه",false),
  (5,17,"روز خبرنگار",false),(6,1,"روز پزشک",false),
  (7,1,"آغاز سال تحصیلی",false),(7,7,"روز آتش نشانی",false),
  (8,13,"روز دانش آموز",false),(9,16,"روز دانشجو",false),
  (10,20,"سالروز شهادت امیرکبیر",false),(11,22,"پیروزی انقلاب اسلامی",true),
  (12,5,"روز مهندس",false),(12,29,"روز ملی شدن صنعت نفت",true)
 ];
 for &(mm,dd,title,holiday) in fixed{if mm==m&&dd==d{result.push(CalendarEvent{title:title.into(),holiday});}}
 // Bundled 1405 holiday data. Do not apply lunar dates to other years.
 if y==1405 {
  if let Ok(events)=serde_json::from_str::<BTreeMap<String,Vec<CalendarEvent>>>(include_str!("../SushiCalendar_Events_1405.json")) {
   if let Some(items)=events.get(&key(y,m,d)){result=items.clone();}
  }
 }
 // Optional year-specific data, e.g. {"1405/01/01":[{"title":"...","holiday":true}]}.
 // User data is kept separate from the source and from personal notes.
 let dataset=std::env::current_exe().ok().and_then(|e|e.parent().map(|x|x.join("SushiCalendar_Events.json")));
 if let Some(path)=dataset{
  if let Ok(content)=fs::read_to_string(path){
   if let Ok(events)=serde_json::from_str::<BTreeMap<String,Vec<CalendarEvent>>>(&content){
    if let Some(items)=events.get(&key(y,m,d)){result=items.clone();}
   }
  }
 }
 result
}
fn g2j(gy:i32,gm:u32,gd:u32)->(i32,u32,u32){let a=[0,31,59,90,120,151,181,212,243,273,304,334];let gy2=if gm>2{gy+1}else{gy};let mut x=355666+365*gy+(gy2+3)/4-(gy2+99)/100+(gy2+399)/400+gd as i32+a[(gm-1)as usize];let mut jy=-1595+33*(x/12053);x%=12053;jy+=4*(x/1461);x%=1461;if x>365{jy+=(x-1)/365;x=(x-1)%365;}if x<186{(jy,(1+x/31)as u32,(1+x%31)as u32)}else{(jy,(7+(x-186)/30)as u32,(1+(x-186)%30)as u32)}}
fn j2g(mut jy:i32,jm:u32,jd:u32)->(i32,u32,u32){jy+=1595;let mut x=-355668+365*jy+(jy/33)*8+(jy%33+3)/4+jd as i32+if jm<7{(jm as i32-1)*31}else{(jm as i32-7)*30+186};let mut gy=400*(x/146097);x%=146097;if x>36524{x-=1;gy+=100*(x/36524);x%=36524;if x>=365{x+=1}}gy+=4*(x/1461);x%=1461;if x>365{gy+=(x-1)/365;x=(x-1)%365}let mut gd=x+1;let leap=(gy%4==0&&gy%100!=0)||gy%400==0;let s=[0,31,if leap{29}else{28},31,30,31,30,31,31,30,31,30,31];let mut gm=1;while gm<=12&&gd>s[gm]{gd-=s[gm];gm+=1}(gy,gm as u32,gd as u32)}
fn mlen(y:i32,m:u32)->u32{if m<=6{31}else if m<=11{30}else{let a=j2g(y,1,1);let b=j2g(y+1,1,1);let x=NaiveDate::from_ymd_opt(a.0,a.1,a.2).unwrap();let z=NaiveDate::from_ymd_opt(b.0,b.1,b.2).unwrap();if(z-x).num_days()==366{30}else{29}}}
fn weekday(y:i32,m:u32,d:u32)->&'static str{let g=j2g(y,m,d);match NaiveDate::from_ymd_opt(g.0,g.1,g.2).unwrap().weekday(){Weekday::Sat=>"شنبه",Weekday::Sun=>"یکشنبه",Weekday::Mon=>"دوشنبه",Weekday::Tue=>"سه‌شنبه",Weekday::Wed=>"چهارشنبه",Weekday::Thu=>"پنجشنبه",Weekday::Fri=>"جمعه"}}


#[cfg(windows)]
fn wstr(s:&str)->Vec<u16>{s.encode_utf16().chain(std::iter::once(0)).collect()}

#[cfg(windows)]
unsafe fn native_note_create()->windows_sys::Win32::Foundation::HWND{
 use windows_sys::Win32::UI::WindowsAndMessaging::*;
 let title=wstr("Sushi Calendar v2.5.95");
 let parent=FindWindowW(std::ptr::null(),title.as_ptr());
 if parent.is_null(){return std::ptr::null_mut();}
 // Load the system RichEdit implementation for per-paragraph formatting.
 let dll=wstr("Msftedit.dll");
 let _rich=windows_sys::Win32::System::LibraryLoader::LoadLibraryW(dll.as_ptr());
 let cls=wstr("RICHEDIT50W");
 let empty=wstr("");
 let style=WS_CHILD|WS_VISIBLE|WS_VSCROLL|WS_TABSTOP|ES_NOHIDESEL as u32|ES_MULTILINE as u32|ES_AUTOVSCROLL as u32|ES_WANTRETURN as u32;
 let ex=WS_EX_CLIENTEDGE|WS_EX_RIGHTSCROLLBAR;
 let h=CreateWindowExW(ex,cls.as_ptr(),empty.as_ptr(),style,0,0,100,100,parent,std::ptr::null_mut(),std::ptr::null_mut(),std::ptr::null());
 if !h.is_null(){
  const EM_SETLIMITTEXT_LOCAL:u32=0x00C5;
  SendMessageW(h,EM_SETLIMITTEXT_LOCAL,1024*1024,0);
 }
 h
}
#[cfg(windows)]
unsafe fn popup_note_dark(h:windows_sys::Win32::Foundation::HWND){
 use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
 if h.is_null(){return;}
 // RichEdit COLORREF is 0x00BBGGRR. Explicitly set both background and default text.
 const EM_SETBKGNDCOLOR:u32=0x0443;
 const EM_SETCHARFORMAT:u32=0x0444;
 const CFM_COLOR:u32=0x40000000;
 #[repr(C)] struct CharFormatW{
  cb_size:u32,dw_mask:u32,dw_effects:u32,y_height:i32,y_offset:i32,
  cr_text_color:u32,b_char_set:u8,b_pitch_and_family:u8,sz_face_name:[u16;32],
 }
 SendMessageW(h,EM_SETBKGNDCOLOR,0,0x002F1808); // matte navy RGB(8,24,47)
 let mut fmt=CharFormatW{cb_size:std::mem::size_of::<CharFormatW>() as u32,
  dw_mask:CFM_COLOR,dw_effects:0,y_height:0,y_offset:0,
  cr_text_color:0x00FFFFFF, // high-contrast white text
  b_char_set:0,b_pitch_and_family:0,sz_face_name:[0;32]};
 // SCF_DEFAULT updates the typing format, SCF_ALL corrects any existing dark text.
 // Avoid repeated calls on every frame: this function runs at editor creation only.
 SendMessageW(h,EM_SETCHARFORMAT,0,&mut fmt as *mut _ as isize);
 SendMessageW(h,EM_SETCHARFORMAT,4,&mut fmt as *mut _ as isize);
}
#[cfg(windows)]
unsafe fn native_note_hide(h:windows_sys::Win32::Foundation::HWND){
 use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow,SW_HIDE};
 if !h.is_null(){ShowWindow(h,SW_HIDE);}
}
#[cfg(windows)]
unsafe fn native_note_show(h:windows_sys::Win32::Foundation::HWND){
 use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow,SW_SHOW};
 if !h.is_null(){ShowWindow(h,SW_SHOW);}
}
#[cfg(windows)]
unsafe fn native_note_set(h:windows_sys::Win32::Foundation::HWND,s:&str){
 use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowTextW,GetWindowTextLengthW,GetWindowTextW};
 if h.is_null(){return}
 let n=GetWindowTextLengthW(h);
 let mut b=vec![0u16;n as usize+1];GetWindowTextW(h,b.as_mut_ptr(),b.len() as i32);
 let cur=String::from_utf16_lossy(&b[..n as usize]);
 if cur!=s{let w=wstr(s);SetWindowTextW(h,w.as_ptr());}
}
#[cfg(windows)]
unsafe fn native_note_get(h:windows_sys::Win32::Foundation::HWND)->String{
 use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW,GetWindowTextW};
 if h.is_null(){return String::new()}
 let n=GetWindowTextLengthW(h);let mut b=vec![0u16;n as usize+1];GetWindowTextW(h,b.as_mut_ptr(),b.len() as i32);String::from_utf16_lossy(&b[..n as usize])
}
// RichEdit supports independent paragraph alignment without changing the
// logical Unicode buffer or the Windows caret/selection behavior.
#[cfg(windows)]
unsafe fn native_note_align(h:windows_sys::Win32::Foundation::HWND,s:&str){
 use windows_sys::Win32::UI::WindowsAndMessaging::SendMessageW;
 if h.is_null(){return;}
 const EM_EXGETSEL:u32=0x0434;
 const EM_EXSETSEL:u32=0x0437;
 const EM_SETPARAFORMAT:u32=0x0447;
 const PFM_ALIGNMENT:u32=0x00000008;
 
 const PFA_LEFT:u16=1;
 const PFA_RIGHT:u16=2;
 #[repr(C)] struct CharRange{cp_min:i32,cp_max:i32}
 #[repr(C)] struct ParaFormat{
  cb_size:u32,dw_mask:u32,w_numbering:u16,w_reserved:u16,
  dx_start_indent:i32,dx_right_indent:i32,dx_offset:i32,w_alignment:u16,
  c_tab_count:i16,rgx_tabs:[i32;32],
 }
 let mut old=CharRange{cp_min:0,cp_max:0};
 SendMessageW(h,EM_EXGETSEL,0,&mut old as *mut _ as isize);
 // RichEdit reports logical line starts in its own character coordinate system.
 // Do not calculate offsets from UTF-8 or split_inclusive: CR/LF normalization
 // makes them differ from the RichEdit buffer, moving formatting to other lines.
 const EM_GETLINECOUNT:u32=0x00BA;
 const EM_LINEINDEX:u32=0x00BB;
 const EM_LINELENGTH:u32=0x00C1;
 let lines:Vec<&str>=s.split('\n').collect();
 let line_count=SendMessageW(h,EM_GETLINECOUNT,0,0).max(0) as usize;
 for i in 0..line_count.min(lines.len()){
  let start=SendMessageW(h,EM_LINEINDEX,i,0) as i32;
  if start<0{continue;}
  let length=SendMessageW(h,EM_LINELENGTH,start as usize,0).max(0) as i32;
  let rtl=lines[i].chars().find_map(|c|{
   if matches!(c as u32,0x0600..=0x06FF|0x0750..=0x077F|0x08A0..=0x08FF){Some(true)}
   else if c.is_ascii_alphabetic(){Some(false)}else{None}
  }).unwrap_or(true);
  // A zero-length selection inside the paragraph changes only that paragraph.
  // Never select across a paragraph separator.
  let pos=start+(length/2);
  let mut range=CharRange{cp_min:pos,cp_max:pos};
  SendMessageW(h,EM_EXSETSEL,0,&mut range as *mut _ as isize);
  let mut fmt=ParaFormat{cb_size:std::mem::size_of::<ParaFormat>() as u32,dw_mask:PFM_ALIGNMENT,w_numbering:0,w_reserved:0,dx_start_indent:0,dx_right_indent:0,dx_offset:0,w_alignment:if rtl{PFA_RIGHT}else{PFA_LEFT},c_tab_count:0,rgx_tabs:[0;32]};
  SendMessageW(h,EM_SETPARAFORMAT,0,&mut fmt as *mut _ as isize);
 }
 SendMessageW(h,EM_EXSETSEL,0,&mut old as *mut _ as isize);
}
#[cfg(windows)]
unsafe fn native_note_place(h:windows_sys::Win32::Foundation::HWND,r:egui::Rect,ppp:f32){
 use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos,GetWindowRect,GetParent,HWND_TOP,SWP_NOACTIVATE};
 use windows_sys::Win32::Graphics::Gdi::MapWindowPoints;
 if h.is_null(){return}
 let x=(r.min.x*ppp).round() as i32;let y=(r.min.y*ppp).round() as i32;
 let w=(r.width()*ppp).round().max(20.) as i32;let hh=(r.height()*ppp).round().max(20.) as i32;
 // Do not reposition a native RichEdit control every frame: doing so can
 // force synchronous Win32 layout/paint while a key is being processed.
 let mut bounds=windows_sys::Win32::Foundation::RECT{left:0,top:0,right:0,bottom:0};
 if GetWindowRect(h,&mut bounds)!=0{
  let parent=GetParent(h);
  if !parent.is_null(){MapWindowPoints(std::ptr::null_mut(),parent,&mut bounds as *mut _ as *mut _,2);}
  if bounds.left==x && bounds.top==y && bounds.right-bounds.left==w && bounds.bottom-bounds.top==hh{return;}
 }
 SetWindowPos(h,HWND_TOP,x,y,w,hh,SWP_NOACTIVATE);
}

fn note_direction_signature(s:&str)->String{
 s.split('\n').map(|line|{
  line.chars().find_map(|c|{
   if matches!(c as u32,0x0600..=0x06FF|0x0750..=0x077F|0x08A0..=0x08FF){Some('R')}
   else if c.is_ascii_alphabetic(){Some('L')}else{None}
  }).unwrap_or('R')
 }).collect()
}
fn has_persian(s:&str)->bool{s.chars().any(|c|matches!(c as u32,0x0600..=0x06FF|0x0750..=0x077F|0x08A0..=0x08FF|0xFB50..=0xFDFF|0xFE70..=0xFEFF))}
fn rtl_note_edit(ui:&mut egui::Ui,text:&mut String,height:f32)->egui::Response{
 // Use egui's real TextEdit instead of a painted fake editor. This restores the
 // native editor interaction model: caret, mouse selection, Ctrl+A/C/X/V,
 // arrows/Home/End/Delete/Backspace and automatic scrolling to the caret.
 // Keep the stored buffer as raw Unicode so cursor/selection indices always
 // refer to the same text that TextEdit edits.
 ui.scope(|ui|{
  ui.visuals_mut().override_text_color=Some(Color32::from_rgb(8,24,47));
  ui.visuals_mut().extreme_bg_color=Color32::WHITE;
  ui.visuals_mut().widgets.inactive.bg_fill=Color32::from_rgb(247,249,252);
  ui.visuals_mut().widgets.hovered.bg_fill=Color32::WHITE;
  ui.visuals_mut().widgets.active.bg_fill=Color32::WHITE;
  let rtl = text.is_empty() || has_persian(text);
  // IMPORTANT: never reshape/reverse the editable buffer. Keep logical Unicode order.
  // Direction/alignment is visual only, so selection/caret indices stay valid.
  egui::ScrollArea::vertical()
   .id_salt("sushi_note_scroll")
   .max_height(height)
   .stick_to_bottom(false)
   .auto_shrink([false,false])
   .show(ui,|ui|{
    ui.add_sized(
     [ui.available_width(),height],
     egui::TextEdit::multiline(text)
      .font(egui::TextStyle::Body)
      .text_color(Color32::from_rgb(8,24,47))
      .horizontal_align(if rtl{egui::Align::RIGHT}else{egui::Align::LEFT})
      .desired_width(f32::INFINITY)
      .desired_rows(7)
      .lock_focus(true)
    )
   }).inner
 }).inner
}
fn light_edit(ui:&mut egui::Ui,text:&mut String,multiline:bool)->egui::Response{
 if multiline{return rtl_note_edit(ui,text,150.);}
 ui.scope(|ui|{
  ui.visuals_mut().override_text_color=Some(Color32::from_rgb(8,24,47));
  ui.visuals_mut().extreme_bg_color=Color32::WHITE;
  ui.visuals_mut().widgets.inactive.bg_fill=Color32::from_rgb(247,249,252);
  ui.visuals_mut().widgets.active.bg_fill=Color32::WHITE;
  let rtl = text.is_empty() || has_persian(text);
  ui.add(egui::TextEdit::singleline(text).text_color(Color32::from_rgb(8,24,47)).horizontal_align(if rtl{egui::Align::RIGHT}else{egui::Align::LEFT}).desired_width(f32::INFINITY))
 }).inner
}
fn navy_button(ui:&mut egui::Ui,label:&str)->egui::Response{
 ui.add(egui::Button::new(RichText::new(fa(label)).color(Color32::WHITE).strong())
  .fill(Color32::from_rgb(28,86,154)).stroke(egui::Stroke::new(1_f32,Color32::from_rgb(55,125,210))).rounding(6.))
}

struct App{y:i32,m:u32,d:u32,s:Store,note:String,rtext:String,rtime:String,holiday:String,repeat:usize,toast:String,edit_note:bool,edit_holiday:bool,holiday_day:u32,tray_day:u32,_tray:Option<TrayIcon>,tray_refresh:Arc<AtomicU8>,menu_open:Option<MenuId>,menu_today:Option<MenuId>,menu_note:Option<MenuId>,menu_exit:Option<MenuId>,was_min:bool,pick_date:bool,pick_y:i32,pick_m:u32,tray_cmd:Arc<AtomicU8>,#[cfg(windows)]native_popup_cmd:Arc<AtomicU8>,#[cfg(windows)]popup_note_request:Arc<AtomicU64>,#[cfg(windows)]native_note:windows_sys::Win32::Foundation::HWND,note_last_aligned:String,#[cfg(windows)]popup_note:windows_sys::Win32::Foundation::HWND,popup_note_last_aligned:String,popup_note_initialized:bool,popup_open:bool,popup_y:i32,popup_m:u32,popup_hide_pending:u8,popup_note_text:String}
impl App{
 fn new(_ctx:&egui::Context)->Self{
  let n=Local::now();let(y,m,d)=g2j(n.year(),n.month(),n.day());
  #[cfg(windows)] {
   use windows_sys::Win32::System::Threading::WaitForSingleObject;
   let repaint=_ctx.clone();
   std::thread::spawn(move||{
    let event=unsafe{windows_sys::Win32::System::Threading::OpenEventW(0x00100000,0,wstr(SINGLE_EVENT).as_ptr())};
    if event.is_null(){return;}
    loop{
     if unsafe{WaitForSingleObject(event,0xFFFFFFFF)}!=0{break;}
     SECOND_LAUNCH.store(true,Ordering::SeqCst);
     restore_native_window();
     repaint.request_repaint();
    }
    unsafe{windows_sys::Win32::Foundation::CloseHandle(event);}
   });
  }
  let tr=tray();
  let (tray_icon,menu_open,menu_today,menu_note,menu_exit)=match tr{
   Some((t,a,b,c,d))=>(Some(t),Some(a),Some(b),Some(c),Some(d)),
   None=>(None,None,None,None,None)
  };
  // Background worker only requests refresh; TrayIcon is !Send.
  let tray_refresh=Arc::new(AtomicU8::new(0));
  let signal=tray_refresh.clone();let repaint=_ctx.clone();
  std::thread::spawn(move||{
   let mut last=ty_now();
   loop{
    std::thread::sleep(std::time::Duration::from_secs(15));
    let current=ty_now();
    if current!=last{signal.store(1,Ordering::SeqCst);repaint.request_repaint();last=current;}
   }
  });
  #[cfg(windows)] let popup_note_request=Arc::new(AtomicU64::new(0));
  #[cfg(windows)] let native_popup_cmd=native_tray_popup::start(popup_note_request.clone());
  #[cfg(windows)] native_tray_popup::start_desktop_widget(popup_note_request.clone());
  // The native popup and desktop widget are on independent Win32 threads.
  // Wake the hidden egui viewport when either sends a note request.
  #[cfg(windows)] {
   let pending=popup_note_request.clone();let wake=_ctx.clone();
   std::thread::spawn(move||loop{
    std::thread::sleep(std::time::Duration::from_millis(80));
    if pending.load(Ordering::SeqCst)!=0{
     STARTUP_HIDDEN.store(false,Ordering::SeqCst);
     restore_native_window();
     wake.send_viewport_cmd(egui::ViewportCommand::Visible(true));
     wake.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
     wake.request_repaint();
    }
   });
  }

  let tray_cmd=Arc::new(AtomicU8::new(0));
  let cmd_click=tray_cmd.clone();let repaint_click=_ctx.clone();
  let last_double=Arc::new(std::sync::Mutex::new(None::<std::time::Instant>));let ld=last_double.clone();
  let click_generation=Arc::new(std::sync::atomic::AtomicU64::new(0));let click_gen_handler=click_generation.clone();
  #[cfg(windows)] let popup_click=native_popup_cmd.clone();
  TrayIconEvent::set_event_handler(Some(move |ev:TrayIconEvent|{
   match ev{
    TrayIconEvent::Click{button:MouseButton::Left,..}=>{diagnostic_log("TRAY_LEFT_CLICK received");
     let recent=ld.lock().ok().and_then(|x|*x).is_some_and(|t|t.elapsed()<std::time::Duration::from_millis(450));
     if !recent{
      let token=click_gen_handler.fetch_add(1,Ordering::SeqCst)+1;
      let generation=click_gen_handler.clone();let command=cmd_click.clone();let repaint=repaint_click.clone();
      #[cfg(windows)] let popup=popup_click.clone();
      std::thread::spawn(move||{
       std::thread::sleep(std::time::Duration::from_millis(350));
       if generation.load(Ordering::SeqCst)==token{diagnostic_log("TRAY_SINGLE_CLICK -> native popup toggle");
        #[cfg(windows)] popup.store(1,Ordering::SeqCst);
        #[cfg(not(windows))] {command.store(4,Ordering::SeqCst);repaint.request_repaint();}}else{diagnostic_log("TRAY_CLICK cancelled as double-click");}
      });
     }
    },
    TrayIconEvent::DoubleClick{button:MouseButton::Left,..}=>{diagnostic_log("TRAY_DOUBLE_CLICK received");if let Ok(mut x)=ld.lock(){*x=Some(std::time::Instant::now());}click_gen_handler.fetch_add(1,Ordering::SeqCst);
     #[cfg(windows)] popup_click.store(2,Ordering::SeqCst);
     #[cfg(windows)] STARTUP_HIDDEN.store(false,Ordering::SeqCst);
     restore_native_window();cmd_click.store(1,Ordering::SeqCst);repaint_click.request_repaint();},
    _=>{}
   }
  }));
  if let (Some(a),Some(b),Some(c),Some(d))=(menu_open.clone(),menu_today.clone(),menu_note.clone(),menu_exit.clone()){
   let cmd_menu=tray_cmd.clone();let repaint_menu=_ctx.clone();
   MenuEvent::set_event_handler(Some(move |ev:MenuEvent|{
    if ev.id==a{#[cfg(windows)] STARTUP_HIDDEN.store(false,Ordering::SeqCst);restore_native_window();cmd_menu.store(1,Ordering::SeqCst);repaint_menu.request_repaint();}
    else if ev.id==b{#[cfg(windows)] STARTUP_HIDDEN.store(false,Ordering::SeqCst);restore_native_window();cmd_menu.store(2,Ordering::SeqCst);repaint_menu.request_repaint();}
    else if ev.id==c{#[cfg(windows)] STARTUP_HIDDEN.store(false,Ordering::SeqCst);restore_native_window();cmd_menu.store(3,Ordering::SeqCst);repaint_menu.request_repaint();}
    else if ev.id==d{std::process::exit(0);}
   }));
  }
  Self{y,m,d,s:load(),note:"".into(),rtext:"".into(),rtime:"09:00".into(),holiday:"".into(),repeat:0,toast:"".into(),edit_note:false,edit_holiday:false,holiday_day:d,tray_day:d,_tray:tray_icon,tray_refresh,menu_open,menu_today,menu_note,menu_exit,was_min:false,pick_date:false,pick_y:y,pick_m:m,tray_cmd,#[cfg(windows)]native_popup_cmd,#[cfg(windows)]popup_note_request,#[cfg(windows)]native_note:std::ptr::null_mut(),note_last_aligned:String::new(),#[cfg(windows)]popup_note:std::ptr::null_mut(),popup_note_last_aligned:String::new(),popup_note_initialized:false,popup_open:false,popup_y:y,popup_m:m,popup_hide_pending:0,popup_note_text:String::new()}
 }
 fn mv(&mut self,n:i32){let q=self.m as i32+n;if q<1{self.m=12;self.y-=1}else if q>12{self.m=1;self.y+=1}else{self.m=q as u32}self.d=1}
 fn year_mv(&mut self,n:i32){self.y+=n;self.d=self.d.min(mlen(self.y,self.m));}
 fn today(&mut self){let n=Local::now();let(y,m,d)=g2j(n.year(),n.month(),n.day());self.y=y;self.m=m;self.d=d}
 fn day(&mut self)->&mut Day{self.s.days.entry(key(self.y,self.m,self.d)).or_default()}
 #[cfg(windows)]fn startup(on:bool){use winreg::{enums::*,RegKey};if let Ok(k)=RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Run",KEY_SET_VALUE){if on{if let Ok(e)=std::env::current_exe(){let cmd=format!("\"{}\" --startup",e.display());let _=k.set_value("SushiCalendar",&cmd);let _=k.delete_value("MRCalendar");}}else{let _=k.delete_value("SushiCalendar");let _=k.delete_value("MRCalendar");}}}
 #[cfg(not(windows))]fn startup(_:bool){}
 fn alarms(&mut self){let n=Local::now();let(y,m,d)=g2j(n.year(),n.month(),n.day());let td=key(y,m,d);let hm=format!("{:02}:{:02}",n.hour(),n.minute());let stamp=format!("{td} {hm}");let mut out=vec![];for(dk,day)in self.s.days.iter_mut(){for r in &mut day.reminders{let active=r.repeat=="روزانه"||dk==&td;if active&&!r.done&&r.time==hm&&r.last!=stamp{out.push(r.text.clone());r.last=stamp.clone();}}}if!out.is_empty(){self.toast=format!("🔔 {}",out.join(" • "));save(&self.s)}}
}
impl eframe::App for App{
 fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame){
  ctx.request_repaint_after(std::time::Duration::from_secs(2));self.alarms();
  // Update tray only on the UI thread; no TrayIcon is moved to a worker.
  let (yy,mm,dd)=ty_now();
  if self.tray_refresh.swap(0,Ordering::SeqCst)!=0 || (yy,mm,dd).2!=self.tray_day{
   if let Some(tr)=self._tray.as_ref(){
    if let Some(ic)=tray_day_icon(dd){let _=tr.set_icon(Some(ic));}
    let tip=format!("\u{202B}{} {} {} {}\u{202C}",weekday(yy,mm,dd),pd(dd),MONTHS[(mm-1)as usize],pd(yy));
    let _=tr.set_tooltip(Some(tip));
   }
   self.tray_day=dd;
  }


  #[cfg(windows)] if SECOND_LAUNCH.swap(false,Ordering::SeqCst){
   STARTUP_HIDDEN.store(false,Ordering::SeqCst);
   self.popup_open=false;
   ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
   ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
   ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
  }
  #[cfg(windows)] {
   let request=self.popup_note_request.swap(0,Ordering::SeqCst);
   if request!=0{
    self.y=(request/10000) as i32;
    self.m=((request/100)%100) as u32;
    self.d=(request%100) as u32;
    self.edit_note=true;
    self.popup_note_initialized=false;
    STARTUP_HIDDEN.store(false,Ordering::SeqCst);
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
   }
  }
  let cmd=self.tray_cmd.swap(0,Ordering::SeqCst);
  if cmd==4{diagnostic_log("MAIN forwarding click to independent native popup");
   #[cfg(windows)] self.native_popup_cmd.store(1,Ordering::SeqCst);
  }
  else if cmd!=0{
   #[cfg(windows)] STARTUP_HIDDEN.store(false,Ordering::SeqCst);
   self.popup_open=false;
   #[cfg(windows)] self.native_popup_cmd.store(2,Ordering::SeqCst);
   if cmd==2{self.today();}
   else if cmd==3{self.today();self.note.clear();self.edit_note=true;}
   ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
   ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
   ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
  }
  #[cfg(windows)] if STARTUP_HIDDEN.load(Ordering::SeqCst){
   ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
   hide_main_native();
  }
  let main_id = ctx.viewport_id();
  let main_close = ctx.input(|i|i.viewport().close_requested());
  if main_close {
   diagnostic_log("MAIN close requested");
   #[cfg(windows)] self.native_popup_cmd.store(2,Ordering::SeqCst);
   self.popup_open=false;
   ctx.send_viewport_cmd_to(egui::ViewportId::from_hash_of("sushi_tray_mini_calendar"),egui::ViewportCommand::Close);
   ctx.send_viewport_cmd_to(main_id,egui::ViewportCommand::CancelClose);
   ctx.send_viewport_cmd_to(main_id,egui::ViewportCommand::Visible(false));
   #[cfg(windows)] hide_main_native();
   // Keep popup state independent; the main window is only hidden to tray.
   ctx.request_repaint();
  }
  let minimized=ctx.input(|i|i.viewport().minimized.unwrap_or(false));
  if minimized&&!self.was_min{ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));}
  // Tray date refresh runs independently while the main viewport is hidden.
  self.was_min=minimized;
  // Tray mini calendar is a native Win32 window managed by its own message loop.
  // Never create an egui child viewport here: hiding the parent can stall its event loop.

  egui::TopBottomPanel::top("head").frame(egui::Frame::default().fill(NAVY).inner_margin(egui::Margin::symmetric(16.,8.))).show(ctx,|ui|{
   let width=ui.available_width();
   let (bar,_) = ui.allocate_exact_size(egui::vec2(width,54.),egui::Sense::hover());
   ui.painter().rect_filled(bar,17.,Color32::from_rgb(5,101,112));
   let left=egui::Rect::from_min_size(bar.min+egui::vec2(9.,8.),egui::vec2(114.,38.));
   let right=egui::Rect::from_min_size(egui::pos2(bar.right()-123.,bar.top()+8.),egui::vec2(114.,38.));
   let mid=egui::Rect::from_center_size(bar.center(),egui::vec2(215.,42.));
   if ui.put(left,egui::Button::new(RichText::new(fa("‹ ماه بعد")).size(15.).color(Color32::WHITE)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).clicked(){self.mv(1);}
   if ui.put(right,egui::Button::new(RichText::new(fa("ماه قبل ›")).size(15.).color(Color32::WHITE)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).clicked(){self.mv(-1);}
   let title=fa(&format!("{}  {}",MONTHS[(self.m-1)as usize],pd(self.y)));
   if ui.put(mid,egui::Button::new(RichText::new(title).size(25.).color(Color32::WHITE)).fill(Color32::TRANSPARENT).stroke(egui::Stroke::NONE)).clicked(){self.pick_y=self.y;self.pick_m=self.m;self.pick_date=true;}
   // Draw arrows as shapes rather than unsupported font glyphs.
   for x in [bar.center().x-146.,bar.center().x+146.]{
    let y=bar.center().y;
    ui.painter().line_segment([egui::pos2(x-5.,y-2.),egui::pos2(x,y+3.)],egui::Stroke::new(1.6_f32,Color32::from_rgb(148,211,217)));
    ui.painter().line_segment([egui::pos2(x,y+3.),egui::pos2(x+5.,y-2.)],egui::Stroke::new(1.6_f32,Color32::from_rgb(148,211,217)));
   }
   ui.horizontal_centered(|ui|{
    ui.spacing_mut().item_spacing.x=8.;
    if ui.small_button(fa("سال قبل")).clicked(){self.year_mv(-1);}
    if ui.small_button(fa("امروز")).clicked(){self.today();}
    if ui.small_button(fa("سال بعد")).clicked(){self.year_mv(1);}
   });
  });
  egui::SidePanel::right("details").default_width(350.).frame(egui::Frame::default().fill(PANEL).inner_margin(18.)).show(ctx,|ui|{
   ui.visuals_mut().override_text_color=Some(Color32::WHITE);ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT),|ui|{
    let g=j2g(self.y,self.m,self.d);let wd=weekday(self.y,self.m,self.d);
    ui.heading(RichText::new(fa(&format!("{} {} {} {}",wd,pd(self.d),MONTHS[(self.m-1)as usize],pd(self.y)))).size(24.).color(Color32::WHITE));
    ui.label(RichText::new(format!("{} {} {}",g.2,GMON[(g.1-1)as usize],g.0)).size(13.).color(Color32::from_rgb(165,180,200)));
    ui.label(RichText::new(fa(&format!("{} {} {} شاهنشاهی",pd(self.d),MONTHS[(self.m-1)as usize],pd(self.y+1180)))).color(Color32::from_rgb(230,190,80)));ui.separator();
    let snap=self.s.days.get(&key(self.y,self.m,self.d)).cloned().unwrap_or_default();
    let occasions=official_events(self.y,self.m,self.d);
    for occasion in occasions{
     let label=if occasion.holiday{format!("★ تعطیل رسمی: {}",occasion.title)}else{format!("• مناسبت: {}",occasion.title)};
     ui.colored_label(if occasion.holiday{Color32::LIGHT_RED}else{Color32::from_rgb(130,220,225)},fa(&label));
    }
    if let Some(h)=snap.holiday{ui.colored_label(Color32::LIGHT_RED,fa(&format!("★ تعطیل: {h}")));}
    for n in snap.notes{ui.group(|ui|{ui.label(fa(&format!("📝 {n}")));});}
    for r in snap.reminders{ui.group(|ui|{ui.label(fa(&format!("🔔 {}  {}",r.time,r.text)));});}
    ui.add_space(10.);ui.heading(fa("افزودن یادداشت"));
    #[cfg(windows)]{
     let (rect,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),150.0),egui::Sense::hover());
     unsafe{if self.native_note.is_null(){self.native_note=native_note_create();popup_note_dark(self.native_note);native_note_set(self.native_note,&self.note);}if true{native_note_show(self.native_note);let current=native_note_get(self.native_note);let sig=note_direction_signature(&current);if sig!=self.note_last_aligned{native_note_align(self.native_note,&current);self.note_last_aligned=sig;}native_note_place(self.native_note,rect,ctx.pixels_per_point());}else{native_note_hide(self.native_note);}}
    }
    #[cfg(not(windows))]light_edit(ui,&mut self.note,true);
    if navy_button(ui,"＋ ذخیره یادداشت").clicked(){
     #[cfg(windows)]unsafe{self.note=native_note_get(self.native_note);}
     if !self.note.trim().is_empty(){let x=std::mem::take(&mut self.note);self.day().notes.push(x);save(&self.s);#[cfg(windows)]unsafe{native_note_set(self.native_note,"");self.note_last_aligned.clear();}}
    }
    ui.separator();ui.heading(fa("یادآوری"));light_edit(ui,&mut self.rtext,false);
    ui.horizontal(|ui|{ui.label(fa("ساعت"));light_edit(ui,&mut self.rtime,false);});
    let rr=["یک‌بار","روزانه","هفتگی","ماهانه"];ui.horizontal_wrapped(|ui|{ui.label(fa("تکرار"));for(i,x)in rr.iter().enumerate(){let fill=if self.repeat==i{Color32::from_rgb(36,112,240)}else{Color32::from_rgb(28,86,154)};if ui.add(egui::Button::new(RichText::new(fa(x)).color(Color32::WHITE)).fill(fill).rounding(5.)).clicked(){self.repeat=i;}}});
    if navy_button(ui,"🔔 ثبت یادآوری").clicked()&&!self.rtext.trim().is_empty(){let text=std::mem::take(&mut self.rtext);let time=self.rtime.clone();let repeat=rr[self.repeat].into();self.day().reminders.push(Reminder{text,time,repeat,done:false,last:"".into()});save(&self.s)}
    let mut st=self.s.startup;if ui.checkbox(&mut st,fa("اجرا همراه ویندوز")).changed(){self.s.startup=st;Self::startup(st);save(&self.s)}
    ui.separator();ui.horizontal_wrapped(|ui|{
     if navy_button(ui,"💾 ذخیره کانفیگ").clicked(){save(&self.s);self.toast=fa("کانفیگ ذخیره شد");}
     if navy_button(ui,"↻ بارگذاری کانفیگ").clicked(){if let Some(v)=reload_config(){self.s=v;self.toast=fa("آخرین کانفیگ بارگذاری شد");}else{self.toast=fa("فایل کانفیگ پیدا نشد");}}
    });
    ui.label(RichText::new(path().to_string_lossy()).size(9.).color(Color32::from_rgb(115,140,165)));
   });
  });
  egui::TopBottomPanel::bottom("credit").frame(egui::Frame::default().fill(NAVY).inner_margin(egui::Margin::symmetric(8.,3.))).show(ctx,|ui|{ui.with_layout(egui::Layout::left_to_right(egui::Align::Center),|ui|{ui.spacing_mut().item_spacing.x=0.;ui.label(RichText::new("By ").size(10.).color(Color32::from_rgb(105,125,150)));ui.label(RichText::new("S").size(11.).strong().color(Color32::WHITE));ui.label(RichText::new("oshiant").size(10.).color(Color32::from_rgb(105,125,150)));});});
  egui::CentralPanel::default().frame(egui::Frame::default().fill(NAVY).inner_margin(16.)).show(ctx,|ui|{
   let gap=7.;
   let cell_w=((ui.available_width()-6.*gap)/7.).clamp(55.,112.);
   let cell_h=(cell_w*0.9).clamp(62.,92.);
   egui::Grid::new("cal").num_columns(7).spacing([gap,gap]).show(ui,|ui|{
    // Visual order left-to-right: Friday ... Saturday; Persian calendar starts at right.
    for col in 0..7{
     let day_index=6-col;
     let color=if col==0{Color32::from_rgb(244,166,83)}else{Color32::from_rgb(115,202,211)};
     ui.add_sized([cell_w,32.],egui::Label::new(RichText::new(fa(WEEK[day_index])).strong().color(color)));
    }
    ui.end_row();
    let g=j2g(self.y,self.m,1);
    let first=NaiveDate::from_ymd_opt(g.0,g.1,g.2).unwrap();
    let start=((first.weekday().num_days_from_monday()+2)%7)as usize;
    let n=Local::now();let today=g2j(n.year(),n.month(),n.day());
    let days=mlen(self.y,self.m)as usize;
    let rows=(start+days+6)/7;
    for row in 0..rows{
     for col in 0..7{
      let sequence=row*7+(6-col);
      if sequence<start||sequence>=start+days{
       ui.allocate_exact_size(egui::vec2(cell_w,cell_h),egui::Sense::hover());
       continue;
      }
      let d=(sequence-start+1)as u32;
      let dd=self.s.days.get(&key(self.y,self.m,d));
      let manual=dd.and_then(|x|x.holiday.as_ref()).is_some();
      let mark=dd.map(|x|!x.notes.is_empty()||!x.reminders.is_empty()).unwrap_or(false);
      let gg=j2g(self.y,self.m,d);
      let red=col==0||manual||official_events(self.y,self.m,d).iter().any(|e|e.holiday);
      let selected=d==self.d;let is_today=(self.y,self.m,d)==today;
      let (rect,r)=ui.allocate_exact_size(egui::vec2(cell_w,cell_h),egui::Sense::click());
      let fill=if is_today{Color32::from_rgb(47,130,141)}else if red{Color32::from_rgb(91,49,16)}else{Color32::from_rgb(45,48,51)};
      ui.painter().rect_filled(rect,10.,fill);
      if selected&&!is_today{ui.painter().rect_stroke(rect,10.,egui::Stroke::new(1.5_f32,BLUE));}
      let mainc=if red{Color32::from_rgb(255,180,101)}else{Color32::from_rgb(222,238,209)};
      ui.painter().text(egui::pos2(rect.center().x,rect.top()+cell_h*0.38),egui::Align2::CENTER_CENTER,pd(d),egui::FontId::proportional((cell_w*0.31).clamp(19.,31.)),mainc);
      let gc=if red{Color32::from_rgb(220,146,75)}else{Color32::from_rgb(156,167,149)};
      let gregorian_line=format!("{} {} ({})",gg.2,GMON[(gg.1-1) as usize],gg.1);
      let gregorian_size=(cell_w / 11.5).clamp(7.0,10.5);
      ui.painter().text(egui::pos2(rect.center().x,rect.bottom()-15.),egui::Align2::CENTER_CENTER,gregorian_line,egui::FontId::proportional(gregorian_size),gc);
      if mark{ui.painter().circle_filled(egui::pos2(rect.right()-9.,rect.top()+9.),3.,Color32::from_rgb(90,220,176));}
      if r.clicked(){self.d=d}if r.double_clicked(){self.d=d;self.note.clear();self.edit_note=true}
      r.context_menu(|ui|{
       self.d=d;
       let is_manual=self.s.days.get(&key(self.y,self.m,d)).and_then(|x|x.holiday.as_ref()).is_some();
       if is_manual{
        if ui.button(fa("حذف تعطیلی")).clicked(){self.s.days.entry(key(self.y,self.m,d)).or_default().holiday=None;save(&self.s);ui.close_menu();}
       }else if col!=0&&ui.button(fa("ثبت تعطیلی")).clicked(){self.holiday.clear();self.holiday_day=d;self.edit_holiday=true;ui.close_menu();}
      });
     }
     ui.end_row();
    }
   });
   if !self.toast.is_empty(){ui.add_space(10.);ui.group(|ui|{ui.colored_label(Color32::YELLOW,&self.toast);if ui.button(fa("تأیید")).clicked(){self.toast.clear()}});}
  });
  if self.pick_date{let mut open=true;egui::Window::new(fa("انتخاب سریع ماه و سال")).open(&mut open).collapsible(false).resizable(false).show(ctx,|ui|{
   ui.horizontal(|ui|{if navy_button(ui,"سال -").clicked(){self.pick_y-=1}ui.label(RichText::new(pd(self.pick_y)).size(20.).color(Color32::WHITE));if navy_button(ui,"سال +").clicked(){self.pick_y+=1}});
   ui.separator();egui::Grid::new("month_picker").num_columns(3).spacing([8.,8.]).show(ui,|ui|{for m in 1..=12u32{let fill=if m==self.pick_m{BLUE}else{Color32::from_rgb(28,86,154)};if ui.add_sized([105.,34.],egui::Button::new(RichText::new(fa(MONTHS[(m-1)as usize])).color(Color32::WHITE)).fill(fill)).clicked(){self.pick_m=m;}if m%3==0{ui.end_row();}}});
   ui.separator();if navy_button(ui,"برو به تاریخ").clicked(){self.y=self.pick_y;self.m=self.pick_m;self.d=1;self.pick_date=false;}
  });if !open{self.pick_date=false}}
  if self.edit_holiday{let mut open=true;egui::Window::new(fa("ثبت تعطیلی")).open(&mut open).collapsible(false).resizable(false).show(ctx,|ui|{ui.label(fa("عنوان تعطیلی"));light_edit(ui,&mut self.holiday,false);if navy_button(ui,"★ ثبت تعطیلی").clicked(){let h=if self.holiday.trim().is_empty(){"تعطیل رسمی".to_string()}else{std::mem::take(&mut self.holiday)};self.s.days.entry(key(self.y,self.m,self.holiday_day)).or_default().holiday=Some(h);save(&self.s);self.edit_holiday=false;}});if !open{self.edit_holiday=false}}
    if self.edit_note{
    let mut open=true;
    egui::Window::new(fa(&format!("یادداشت — {} {} {}",self.d,MONTHS[(self.m-1)as usize],self.y)))
     .open(&mut open).collapsible(false).resizable(true).default_width(560.)
     .show(ctx,|ui|{
       ui.label(fa("یادداشت این روز"));
       if let Some(day)=self.s.days.get(&key(self.y,self.m,self.d)){
        for note in &day.notes {ui.label(fa(note));}
       }

       ui.set_min_width(520.);
       #[cfg(windows)]{
         let (rect,_)=ui.allocate_exact_size(egui::vec2(ui.available_width(),170.),egui::Sense::hover());
         unsafe{
           if self.popup_note.is_null(){self.popup_note=native_note_create();popup_note_dark(self.popup_note);}
           if !self.popup_note_initialized{self.popup_note_text.clear();native_note_set(self.popup_note,&self.popup_note_text);self.popup_note_initialized=true;self.popup_note_last_aligned.clear();}
           let current=native_note_get(self.popup_note);
           let sig=note_direction_signature(&current);if sig!=self.popup_note_last_aligned{native_note_align(self.popup_note,&current);self.popup_note_last_aligned=sig;}
           native_note_place(self.popup_note,rect,ctx.pixels_per_point());native_note_show(self.popup_note);
         }
       }
       #[cfg(not(windows))]{light_edit(ui,&mut self.note,true);}
       if navy_button(ui,"ذخیره").clicked(){
         #[cfg(windows)]unsafe{self.popup_note_text=native_note_get(self.popup_note);}
         if !self.popup_note_text.trim().is_empty(){let x=std::mem::take(&mut self.popup_note_text);self.day().notes.push(x);save(&self.s);self.edit_note=false;}
       }
     });
    if !open{self.edit_note=false;}
    if !self.edit_note{
      #[cfg(windows)]unsafe{native_note_hide(self.popup_note);}
      self.popup_note_initialized=false;
    }
   }else{
     #[cfg(windows)]unsafe{native_note_hide(self.popup_note);}
     self.popup_note_initialized=false;
   }
 }
}
fn main(){
 diagnostic_log("START Sushi Calendar 2.5.83");
 let startup_launch=std::env::args().any(|arg|arg=="--startup" || arg=="--tray" || arg=="--hidden");
 #[cfg(windows)] let _single_guard=match single_instance_guard(startup_launch){Some(g)=>g,None=>return};
 #[cfg(windows)] if startup_launch {
  STARTUP_HIDDEN.store(true,Ordering::SeqCst);
  // Win10 can show a GUI startup window despite the initial viewport visibility flag.
  // Keep it hidden until an explicit tray/menu/second-launch request arrives.
  std::thread::spawn(||{
   for _ in 0..100 {
    if !STARTUP_HIDDEN.load(Ordering::SeqCst){break;}
    hide_main_native();
    std::thread::sleep(std::time::Duration::from_millis(100));
   }
  });
 }
 let opt=eframe::NativeOptions{viewport:egui::ViewportBuilder::default().with_visible(!startup_launch).with_inner_size([1180.,760.]).with_min_inner_size([1020.,660.]).with_icon(egui::IconData{rgba:image::load_from_memory(include_bytes!("../assets/logo.png")).unwrap().into_rgba8().into_raw(),width:256,height:256}),..Default::default()};let _=eframe::run_native("Sushi Calendar v2.5.95",opt,Box::new(|cc|{fonts(&cc.egui_ctx);let mut v=egui::Visuals::dark();v.panel_fill=NAVY;cc.egui_ctx.set_visuals(v);Ok(Box::new(App::new(&cc.egui_ctx)))}));}
