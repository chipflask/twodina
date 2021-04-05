use ruruby::{self, Block, ContextRef, GlobalsRef, ISeqRef, Value, VMRef};

pub fn new_interpreter() -> VMRef {
    let mut globals = GlobalsRef::new_globals();
    let vm = globals.create_main_fiber();
    let vals: Vec<Value> = vec!["script.rb"]
        .iter()
        .map(|x| Value::string(*x))
        .collect();
    let argv = Value::array_from(vals);
    globals.set_toplevel_constant("ARGV", argv);

    vm
}

pub fn context(vm: VMRef) -> ContextRef {
    ContextRef::new_heap(
        vm.globals.main_object,
        Block::None,
        ISeqRef::default(),
        None,
    )
}
