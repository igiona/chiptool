# ! [no_std] # ! [doc = "Peripheral access API (generated using chiptool v0.1.0 (6362222 2024-09-23))"] # [derive (Copy , Clone , Debug , PartialEq , Eq)] pub enum Interrupt { # [doc = "0 - POWER_CLOCK"] POWER_CLOCK = 0 , } unsafe impl cortex_m :: interrupt :: InterruptNumber for Interrupt { # [inline (always)] fn number (self) -> u16 { self as u16 } } # [cfg (feature = "rt")] mod _vectors { extern "C" { fn POWER_CLOCK () ; } pub union Vector { _handler : unsafe extern "C" fn () , _reserved : u32 , } # [link_section = ".vector_table.interrupts"] # [no_mangle] pub static __INTERRUPTS : [Vector ; 1] = [Vector { _handler : POWER_CLOCK } ,] ; } # [doc = "Clock control"] pub const CLOCK : clock :: Clock = unsafe { clock :: Clock :: from_ptr (0x4000_0000usize as _) } ; # [doc = r" Number available in the NVIC for configuring priority"] # [cfg (feature = "rt")] pub const NVIC_PRIO_BITS : u8 = 3 ; # [cfg (feature = "rt")] pub use cortex_m_rt :: interrupt ; # [cfg (feature = "rt")] pub use Interrupt as interrupt ; pub mod clock { # [doc = "Clock control"] # [derive (Copy , Clone , Eq , PartialEq)] pub struct Clock { ptr : * mut u8 } unsafe impl Send for Clock { } unsafe impl Sync for Clock { } impl Clock { # [inline (always)] pub const unsafe fn from_ptr (ptr : * mut ()) -> Self { Self { ptr : ptr as _ , } } # [inline (always)] pub const fn as_ptr (& self) -> * mut () { self . ptr as _ } # [doc = "Enable interrupt"] # [inline (always)] pub const fn intenset (self) -> crate :: common :: Reg < regs :: Inten , crate :: common :: RW > { unsafe { crate :: common :: Reg :: from_ptr (self . ptr . add (0x0304usize) as _) } } # [doc = "Disable interrupt"] # [inline (always)] pub const fn intenclr (self) -> crate :: common :: Reg < regs :: Inten , crate :: common :: RW > { unsafe { crate :: common :: Reg :: from_ptr (self . ptr . add (0x0308usize) as _) } } } pub mod regs { # [doc = "Disable interrupt"] # [repr (transparent)] # [derive (Copy , Clone , Eq , PartialEq)] pub struct Inten (pub u32) ; impl Inten { # [doc = "Write '1' to disable interrupt for event HFCLKSTARTED"] # [inline (always)] pub const fn hfclkstarted (& self) -> bool { let val = (self . 0 >> 0usize) & 0x01 ; val != 0 } # [doc = "Write '1' to disable interrupt for event HFCLKSTARTED"] # [inline (always)] pub fn set_hfclkstarted (& mut self , val : bool) { self . 0 = (self . 0 & ! (0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize) ; } } impl Default for Inten { # [inline (always)] fn default () -> Inten { Inten (0) } } } } pub mod common { use core :: marker :: PhantomData ; # [derive (Copy , Clone , PartialEq , Eq)] pub struct RW ; # [derive (Copy , Clone , PartialEq , Eq)] pub struct R ; # [derive (Copy , Clone , PartialEq , Eq)] pub struct W ; mod sealed { use super ::*; pub trait Access { } impl Access for R { } impl Access for W { } impl Access for RW { } } pub trait Access : sealed :: Access + Copy { } impl Access for R { } impl Access for W { } impl Access for RW { } pub trait Read : Access { } impl Read for RW { } impl Read for R { } pub trait Write : Access { } impl Write for RW { } impl Write for W { } # [derive (Copy , Clone , PartialEq , Eq)] pub struct Reg < T : Copy , A : Access > { ptr : * mut u8 , phantom : PhantomData <* mut (T , A) >, } unsafe impl < T : Copy , A : Access > Send for Reg < T , A > { } unsafe impl < T : Copy , A : Access > Sync for Reg < T , A > { } impl < T : Copy , A : Access > Reg < T , A > { # [allow (clippy :: missing_safety_doc)] # [inline (always)] pub const unsafe fn from_ptr (ptr : * mut T) -> Self { Self { ptr : ptr as _ , phantom : PhantomData , } } # [inline (always)] pub const fn as_ptr (& self) -> * mut T { self . ptr as _ } } impl < T : Copy , A : Read > Reg < T , A > { # [inline (always)] pub fn read (& self) -> T { unsafe { (self . ptr as * mut T) . read_volatile () } } } impl < T : Copy , A : Write > Reg < T , A > { # [inline (always)] pub fn write_value (& self , val : T) { unsafe { (self . ptr as * mut T) . write_volatile (val) } } } impl < T : Default + Copy , A : Write > Reg < T , A > { # [inline (always)] pub fn write < R > (& self , f : impl FnOnce (& mut T) -> R) -> R { let mut val = Default :: default () ; let res = f (& mut val) ; self . write_value (val) ; res } } impl < T : Copy , A : Read + Write > Reg < T , A > { # [inline (always)] pub fn modify < R > (& self , f : impl FnOnce (& mut T) -> R) -> R { let mut val = self . read () ; let res = f (& mut val) ; self . write_value (val) ; res } } }