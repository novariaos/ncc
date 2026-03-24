use std::ffi::c_char;
use std::os::raw::c_int;

unsafe extern "C" {
    fn nvma_assemble(
        asm_text: *const c_char,
        asm_len: usize,
        out_buf: *mut *mut u8,
        out_len: *mut usize,
    ) -> c_int;

    fn nvma_free(buf: *mut u8);
}

pub fn assemble(asm_text: &str) -> Result<Vec<u8>, String> {
    let mut out_buf: *mut u8 = std::ptr::null_mut();
    let mut out_len: usize = 0;

    let ret = unsafe {
        nvma_assemble(
            asm_text.as_ptr() as *const c_char,
            asm_text.len(),
            &mut out_buf,
            &mut out_len,
        )
    };

    if ret != 0 {
        return Err(format!("nvma_assemble failed with code {ret}"));
    }

    if out_buf.is_null() || out_len == 0 {
        return Err("nvma_assemble returned empty output".into());
    }

    let result = unsafe { std::slice::from_raw_parts(out_buf, out_len).to_vec() };
    unsafe {
        nvma_free(out_buf);
    }
    Ok(result)
}
