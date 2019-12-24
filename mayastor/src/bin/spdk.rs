//! The most thin ever rust wrapper around SPDK. This is used for automated
//! and manual testing if running mayastor with all its whistels and blows
//! is not possible or desireable and all what is needed is to run SPDK with
//! particular configuration file (i.e. nvmf target for testing).

extern crate libc;

use libc::{c_char, c_int};
use spdk_sys::{
    spdk_app_fini,
    spdk_app_opts,
    spdk_app_opts_init,
    spdk_app_parse_args,
    spdk_app_start,
    spdk_app_stop,
};
use std::{
    env,
    ffi::{c_void, CString},
    io::{Error, ErrorKind},
    iter::Iterator,
    ptr::null_mut,
    time::Duration,
    vec::Vec,
};

mayastor::CPS_INIT!();

fn main() -> Result<(), std::io::Error> {
    let args = env::args()
        .map(|arg| CString::new(arg).unwrap())
        .collect::<Vec<CString>>();
    let mut c_args = args
        .iter()
        .map(|arg| arg.as_ptr())
        .collect::<Vec<*const c_char>>();
    c_args.push(std::ptr::null());

    let mut opts: spdk_app_opts = Default::default();

    unsafe {
        spdk_app_opts_init(&mut opts as *mut spdk_app_opts);

        if spdk_app_parse_args(
            (c_args.len() as c_int) - 1,
            c_args.as_ptr() as *mut *mut i8,
            &mut opts,
            null_mut(), // extra short options i.e. "f:S:"
            null_mut(), // extra long options
            None,       // extra options parse callback
            None,       // usage
        ) != spdk_sys::SPDK_APP_PARSE_ARGS_SUCCESS
        {
            return Err(Error::new(
                ErrorKind::Other,
                "Parsing arguments failed",
            ));
        }
    }

    opts.name = CString::new("spdk".to_owned()).unwrap().into_raw();
    opts.shutdown_cb = Some(spdk_shutdown_cb);

    let rc = unsafe {
        let rc = spdk_app_start(&mut opts, Some(app_start_cb), null_mut());
        // this will remove shm file in /dev/shm and do other cleanups
        spdk_app_fini();
        rc
    };
    if rc != 0 {
        Err(Error::new(
            ErrorKind::Other,
            format!("spdk failed with error {}", rc),
        ))
    } else {
        Ok(())
    }
}

extern "C" fn spdk_shutdown_cb() {
    unsafe { spdk_app_stop(0) };
}

extern "C" fn developer_delay(_ctx: *mut c_void) -> i32 {
    std::thread::sleep(Duration::from_millis(1));
    0
}

extern "C" fn app_start_cb(_arg: *mut c_void) {
    // use in cases when you want to burn less cpu and speed does not matter
    if let Some(_key) = env::var_os("DELAY") {
        println!("*** Delaying reactor every 1000us ***");
        unsafe {
            spdk_sys::spdk_poller_register(
                Some(developer_delay),
                std::ptr::null_mut(),
                1000,
            )
        };
    }
}