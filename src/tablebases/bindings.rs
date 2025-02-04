/* automatically generated by rust-bindgen 0.60.1 */

pub const _VCRT_COMPILER_PREPROCESSOR: u32 = 1;
pub const _SAL_VERSION: u32 = 20;
pub const __SAL_H_VERSION: u32 = 180000000;
pub const _USE_DECLSPECS_FOR_SAL: u32 = 0;
pub const _USE_ATTRIBUTES_FOR_SAL: u32 = 0;
pub const _CRT_PACKING: u32 = 8;
pub const _HAS_EXCEPTIONS: u32 = 1;
pub const _STL_LANG: u32 = 0;
pub const _HAS_CXX17: u32 = 0;
pub const _HAS_CXX20: u32 = 0;
pub const _HAS_CXX23: u32 = 0;
pub const _HAS_NODISCARD: u32 = 0;
pub const WCHAR_MIN: u32 = 0;
pub const WCHAR_MAX: u32 = 65535;
pub const WINT_MIN: u32 = 0;
pub const WINT_MAX: u32 = 65535;
pub const TB_VALUE_PAWN: u32 = 100;
pub const TB_VALUE_MATE: u32 = 32000;
pub const TB_VALUE_INFINITE: u32 = 32767;
pub const TB_VALUE_DRAW: u32 = 0;
pub const TB_MAX_MATE_PLY: u32 = 255;
pub const __bool_true_false_are_defined: u32 = 1;
pub const true_: u32 = 1;
pub const false_: u32 = 0;
pub const TB_MAX_MOVES: u32 = 193;
pub const TB_MAX_CAPTURES: u32 = 64;
pub const TB_MAX_PLY: u32 = 256;
pub const TB_CASTLING_K: u32 = 1;
pub const TB_CASTLING_Q: u32 = 2;
pub const TB_CASTLING_k: u32 = 4;
pub const TB_CASTLING_q: u32 = 8;
pub const TB_LOSS: u32 = 0;
pub const TB_BLESSED_LOSS: u32 = 1;
pub const TB_DRAW: u32 = 2;
pub const TB_CURSED_WIN: u32 = 3;
pub const TB_WIN: u32 = 4;
pub const TB_PROMOTES_NONE: u32 = 0;
pub const TB_PROMOTES_QUEEN: u32 = 1;
pub const TB_PROMOTES_ROOK: u32 = 2;
pub const TB_PROMOTES_BISHOP: u32 = 3;
pub const TB_PROMOTES_KNIGHT: u32 = 4;
pub const TB_RESULT_WDL_MASK: u32 = 15;
pub const TB_RESULT_TO_MASK: u32 = 1008;
pub const TB_RESULT_FROM_MASK: u32 = 64512;
pub const TB_RESULT_PROMOTES_MASK: u32 = 458752;
pub const TB_RESULT_EP_MASK: u32 = 524288;
pub const TB_RESULT_DTZ_MASK: u32 = 4293918720;
pub const TB_RESULT_WDL_SHIFT: u32 = 0;
pub const TB_RESULT_TO_SHIFT: u32 = 4;
pub const TB_RESULT_FROM_SHIFT: u32 = 10;
pub const TB_RESULT_PROMOTES_SHIFT: u32 = 16;
pub const TB_RESULT_EP_SHIFT: u32 = 19;
pub const TB_RESULT_DTZ_SHIFT: u32 = 20;
pub const TB_RESULT_FAILED: u32 = 4294967295;
pub type va_list = *mut ::std::os::raw::c_char;
extern "C" {
    pub fn __va_start(arg1: *mut *mut ::std::os::raw::c_char, ...);
}
pub type size_t = ::std::os::raw::c_ulonglong;
pub type __vcrt_bool = bool;
pub type wchar_t = ::std::os::raw::c_ushort;
extern "C" {
    pub fn __security_init_cookie();
}
extern "C" {
    pub fn __security_check_cookie(_StackCookie: usize);
}
extern "C" {
    pub fn __report_gsfailure(_StackCookie: usize);
}
extern "C" {
    pub static mut __security_cookie: usize;
}
pub type int_least8_t = ::std::os::raw::c_schar;
pub type int_least16_t = ::std::os::raw::c_short;
pub type int_least32_t = ::std::os::raw::c_int;
pub type int_least64_t = ::std::os::raw::c_longlong;
pub type uint_least8_t = ::std::os::raw::c_uchar;
pub type uint_least16_t = ::std::os::raw::c_ushort;
pub type uint_least32_t = ::std::os::raw::c_uint;
pub type uint_least64_t = ::std::os::raw::c_ulonglong;
pub type int_fast8_t = ::std::os::raw::c_schar;
pub type int_fast16_t = ::std::os::raw::c_int;
pub type int_fast32_t = ::std::os::raw::c_int;
pub type int_fast64_t = ::std::os::raw::c_longlong;
pub type uint_fast8_t = ::std::os::raw::c_uchar;
pub type uint_fast16_t = ::std::os::raw::c_uint;
pub type uint_fast32_t = ::std::os::raw::c_uint;
pub type uint_fast64_t = ::std::os::raw::c_ulonglong;
pub type intmax_t = ::std::os::raw::c_longlong;
pub type uintmax_t = ::std::os::raw::c_ulonglong;
extern "C" {
    pub static index64: [::std::os::raw::c_int; 64usize];
}
extern "C" {
    pub fn tb_init_impl(_path: *const ::std::os::raw::c_char) -> bool;
}
extern "C" {
    pub fn tb_probe_wdl_impl(
        _white: u64,
        _black: u64,
        _kings: u64,
        _queens: u64,
        _rooks: u64,
        _bishops: u64,
        _knights: u64,
        _pawns: u64,
        _ep: ::std::os::raw::c_uint,
        _turn: bool,
    ) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn tb_probe_root_impl(
        _white: u64,
        _black: u64,
        _kings: u64,
        _queens: u64,
        _rooks: u64,
        _bishops: u64,
        _knights: u64,
        _pawns: u64,
        _rule50: ::std::os::raw::c_uint,
        _ep: ::std::os::raw::c_uint,
        _turn: bool,
        _results: *mut ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub static mut TB_LARGEST: ::std::os::raw::c_uint;
}
extern "C" {
    pub fn tb_init(_path: *const ::std::os::raw::c_char) -> bool;
}
extern "C" {
    pub fn tb_free();
}
extern "C" {
    pub fn tb_probe_wdl(
        _white: u64,
        _black: u64,
        _kings: u64,
        _queens: u64,
        _rooks: u64,
        _bishops: u64,
        _knights: u64,
        _pawns: u64,
        _rule50: ::std::os::raw::c_uint,
        _castling: ::std::os::raw::c_uint,
        _ep: ::std::os::raw::c_uint,
        _turn: bool,
    ) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn tb_probe_root(
        _white: u64,
        _black: u64,
        _kings: u64,
        _queens: u64,
        _rooks: u64,
        _bishops: u64,
        _knights: u64,
        _pawns: u64,
        _rule50: ::std::os::raw::c_uint,
        _castling: ::std::os::raw::c_uint,
        _ep: ::std::os::raw::c_uint,
        _turn: bool,
        _results: *mut ::std::os::raw::c_uint,
    ) -> ::std::os::raw::c_uint;
}
pub type TbMove = u16;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TbRootMove {
    pub move_: TbMove,
    pub pv: [TbMove; 256usize],
    pub pvSize: ::std::os::raw::c_uint,
    pub tbScore: i32,
    pub tbRank: i32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TbRootMoves {
    pub size: ::std::os::raw::c_uint,
    pub moves: [TbRootMove; 193usize],
}
extern "C" {
    pub fn tb_probe_root_dtz(
        _white: u64,
        _black: u64,
        _kings: u64,
        _queens: u64,
        _rooks: u64,
        _bishops: u64,
        _knights: u64,
        _pawns: u64,
        _rule50: ::std::os::raw::c_uint,
        _castling: ::std::os::raw::c_uint,
        _ep: ::std::os::raw::c_uint,
        _turn: bool,
        hasRepeated: bool,
        useRule50: bool,
        _results: *mut TbRootMoves,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn tb_probe_root_wdl(
        _white: u64,
        _black: u64,
        _kings: u64,
        _queens: u64,
        _rooks: u64,
        _bishops: u64,
        _knights: u64,
        _pawns: u64,
        _rule50: ::std::os::raw::c_uint,
        _castling: ::std::os::raw::c_uint,
        _ep: ::std::os::raw::c_uint,
        _turn: bool,
        useRule50: bool,
        _results: *mut TbRootMoves,
    ) -> ::std::os::raw::c_int;
}
