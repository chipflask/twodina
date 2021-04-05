use ruruby::{self, *};

pub fn new_interpreter() -> VMRef {
    let mut globals = GlobalsRef::new_globals();
    let vm = globals.create_main_fiber();
    let vals: Vec<Value> = vec!["script.rb"].iter()
                                            .map(|x| Value::string(*x))
                                            .collect();
    let argv = Value::array_from(vals);
    globals.set_toplevel_constant("ARGV", argv);

    // Patch with custom functions.
    let class = Module::class_under_object();
    // BuiltinClass::set_toplevel_constant() is private.
    BuiltinClass::object().set_const_by_str("ScriptCore", class.into());
    class.add_builtin_class_method("say", say);

    vm
}

pub fn context(vm: VMRef) -> ContextRef {
    ContextRef::new_heap(vm.globals.main_object,
                         Block::None,
                         ISeqRef::default(),
                         None)
}

fn say(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 2)?;

    eprintln!("Hello, from Rust.  Called with: {:?}", args);

    if args.len() >= 1 {
        // Extract first arg.
        let mut arg0 = args[0];
        let s = arg0.expect_string("first argument")?;
        // Print it.
        eprintln!("Message: {}", s);
    }

    if args.len() >= 2 {
        // Extract second arg.
        let num = args[1].expect_integer("second argument")?;
        // Return incremented.
        return Ok(Value::integer(num + 1));
    }

    Ok(Value::nil())
}
