fn main(){
 println!("cargo:rerun-if-changed=assets/SushiCalendar.ico");
 #[cfg(windows)] {
  let mut r=winres::WindowsResource::new();
  r.set_icon("assets/SushiCalendar.ico");
  r.compile().expect("Failed to embed Sushi Calendar icon into Windows EXE");
 }
}
