pub type FContext = *mut u8;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub struct Transfer {
    pub fctx: FContext,
    pub data: *mut u8,
}

extern "C" {
    pub fn jump_fcontext(to: FContext, vp: *mut u8) -> Transfer;
    pub fn make_fcontext(sp: *mut u8, size: usize, function: extern "C" fn(Transfer)) -> FContext;
    pub fn ontop_fcontext(
        to: FContext,
        vp: *mut u8,
        function: extern "C" fn(Transfer) -> Transfer,
    ) -> Transfer;
}
