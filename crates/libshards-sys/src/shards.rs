/* automatically generated by rust-bindgen 0.66.1 */

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ShardsSlice {
    pub ptr: *mut ::std::os::raw::c_void,
    pub len: usize,
}
#[test]
fn bindgen_test_layout_ShardsSlice() {
    const UNINIT: ::std::mem::MaybeUninit<ShardsSlice> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<ShardsSlice>(),
        16usize,
        concat!("Size of: ", stringify!(ShardsSlice))
    );
    assert_eq!(
        ::std::mem::align_of::<ShardsSlice>(),
        8usize,
        concat!("Alignment of ", stringify!(ShardsSlice))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).ptr) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(ShardsSlice),
            "::",
            stringify!(ptr)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).len) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(ShardsSlice),
            "::",
            stringify!(len)
        )
    );
}
pub const ShardsParseError_Invalid: ShardsParseError = 0;
pub const ShardsParseError_BadToken: ShardsParseError = 1;
pub type ShardsParseError = ::std::os::raw::c_uint;
pub const ShardsAstState_Errors: ShardsAstState = 0;
pub const ShardsAstState_Tokens: ShardsAstState = 1;
pub type ShardsAstState = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ShardsAst {
    pub state: ShardsAstState,
    pub infos: ShardsAst__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union ShardsAst__bindgen_ty_1 {
    pub error: ShardsParseError,
    pub datas: ShardsSlice,
}
#[test]
fn bindgen_test_layout_ShardsAst__bindgen_ty_1() {
    const UNINIT: ::std::mem::MaybeUninit<ShardsAst__bindgen_ty_1> =
        ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<ShardsAst__bindgen_ty_1>(),
        16usize,
        concat!("Size of: ", stringify!(ShardsAst__bindgen_ty_1))
    );
    assert_eq!(
        ::std::mem::align_of::<ShardsAst__bindgen_ty_1>(),
        8usize,
        concat!("Alignment of ", stringify!(ShardsAst__bindgen_ty_1))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).error) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(ShardsAst__bindgen_ty_1),
            "::",
            stringify!(error)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).datas) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(ShardsAst__bindgen_ty_1),
            "::",
            stringify!(datas)
        )
    );
}
#[test]
fn bindgen_test_layout_ShardsAst() {
    const UNINIT: ::std::mem::MaybeUninit<ShardsAst> = ::std::mem::MaybeUninit::uninit();
    let ptr = UNINIT.as_ptr();
    assert_eq!(
        ::std::mem::size_of::<ShardsAst>(),
        24usize,
        concat!("Size of: ", stringify!(ShardsAst))
    );
    assert_eq!(
        ::std::mem::align_of::<ShardsAst>(),
        8usize,
        concat!("Alignment of ", stringify!(ShardsAst))
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).state) as usize - ptr as usize },
        0usize,
        concat!(
            "Offset of field: ",
            stringify!(ShardsAst),
            "::",
            stringify!(state)
        )
    );
    assert_eq!(
        unsafe { ::std::ptr::addr_of!((*ptr).infos) as usize - ptr as usize },
        8usize,
        concat!(
            "Offset of field: ",
            stringify!(ShardsAst),
            "::",
            stringify!(infos)
        )
    );
}
