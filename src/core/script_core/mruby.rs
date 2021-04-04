use std::ffi::CStr;
use std::sync::Mutex;

use artichoke::backend::{mrb_get_args, unwrap_interpreter};
use artichoke::backend::convert::implicitly_convert_to_int;
use artichoke::backend::error;
use artichoke::backend::extn::prelude::*;
use lazy_static::lazy_static;
use super::ScriptCore;

const SCRIPT_CORE_CSTR: &CStr = cstr::cstr!("ScriptCore");

pub fn init(interp: &mut Artichoke) -> InitializeResult<()> {
    interp.def_file_for_type::<_, ScriptCoreFile>("script_core.rb")?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct ScriptCoreFile {
    // Ensure this type cannot be constructed.
    _private: (),
}

impl File for ScriptCoreFile {
    type Artichoke = Artichoke;
    type Error = Error;

    // This gets called when Ruby code requires the module.
    fn require(interp: &mut Self::Artichoke) -> Result<(), Self::Error> {
        if interp.is_module_defined::<ScriptCore>() {
            return Ok(())
        }

        // Define the module.
        let spec = module::Spec::new(interp, "ScriptCore", SCRIPT_CORE_CSTR, None)?;
        module::Builder::for_spec(interp, &spec)
            .add_self_method("inc", inc, sys::mrb_args_opt(1))?
            .define()?;
        interp.def_module::<ScriptCore>(spec)?;

        eprintln!("Patched ScriptCore onto interpreter");
        Ok(())
    }
}

lazy_static! {
    static ref GLOBAL_COUNT: Mutex<i64> = Mutex::new(0);
}

unsafe extern "C" fn inc(mrb: *mut sys::mrb_state, _slf: sys::mrb_value) -> sys::mrb_value {
    // Extract parameters.
    let num = mrb_get_args!(mrb, optional = 1);
    unwrap_interpreter!(mrb, to => interp);
    let num = num.map(Value::from);
    eprintln!("inc called in Rust; arguments: num={:?}", num);

    // Acquire lock on our state.
    let mut count = GLOBAL_COUNT.lock().expect("failed to acquire lock of GLOBAL_COUNT");

    // Convert argument to Rust value.
    let inc_by = match num {
        None => 1,
        Some(v) => {
            match implicitly_convert_to_int(&mut *interp, v) {
                Ok(n) => n,
                Err(error) => error::raise(interp, error),
            }
        }
    };

    // Do the actual work of the function.
    *count += inc_by;
    eprintln!("after inc; GLOBAL_COUNT={}", count);

    // Return value.
    interp.convert(*count).inner()
}
