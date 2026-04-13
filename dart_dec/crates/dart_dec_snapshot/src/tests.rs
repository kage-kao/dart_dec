#[cfg(test)]
mod tests {
    use crate::*;
    use crate::types::*;
    use crate::stubs::*;
    use crate::object_pool;
    use crate::class_table;
    use crate::string_table;

    #[test]
    fn test_snapshot_addr_display() {
        let addr = SnapshotAddr(0x1234);
        assert_eq!(format!("{}", addr), "0x1234");
    }

    #[test]
    fn test_function_kind_display() {
        assert_eq!(format!("{}", FunctionKind::RegularFunction), "regular");
        assert_eq!(format!("{}", FunctionKind::Getter), "getter");
        assert_eq!(format!("{}", FunctionKind::Constructor), "constructor");
        assert_eq!(format!("{}", FunctionKind::AsyncClosure), "async_closure");
    }

    #[test]
    fn test_stub_kind_display() {
        assert_eq!(format!("{}", StubKind::AllocateObject), "AllocateObject");
        assert_eq!(format!("{}", StubKind::NullCheck), "NullCheck");
    }

    #[test]
    fn test_stub_recognizer() {
        let mut recognizer = StubRecognizer::new();
        let mut stubs = std::collections::HashMap::new();
        stubs.insert(
            "AllocateObjectStub".to_string(),
            "pattern:aarch64:d10043ff".to_string(),
        );
        recognizer.load_from_profile(&stubs);

        let bytes = [0xd1, 0x00, 0x43, 0xff];
        let result = recognizer.recognize(&bytes);
        assert_eq!(result, Some(StubKind::AllocateObject));
    }

    #[test]
    fn test_stub_recognizer_no_match() {
        let recognizer = StubRecognizer::new();
        let bytes = [0x00, 0x00, 0x00, 0x00];
        assert!(recognizer.recognize(&bytes).is_none());
    }

    #[test]
    fn test_object_pool_new() {
        let pool = object_pool::ObjectPool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_object_pool_insert() {
        let mut pool = object_pool::ObjectPool::new();
        pool.insert(SnapshotAddr(0x100), DartObject::Mint(42));
        pool.insert(SnapshotAddr(0x108), DartObject::Null);
        assert_eq!(pool.len(), 2);
        assert!(!pool.is_empty());
    }

    #[test]
    fn test_object_pool_get() {
        let mut pool = object_pool::ObjectPool::new();
        pool.insert(SnapshotAddr(0x100), DartObject::Double(3.14));
        let obj = pool.get(&SnapshotAddr(0x100));
        assert!(matches!(obj, Some(DartObject::Double(v)) if (v - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_object_pool_functions() {
        let mut pool = object_pool::ObjectPool::new();
        pool.insert(
            SnapshotAddr(0x100),
            DartObject::Function(DartFunction {
                addr: SnapshotAddr(0x100),
                name: "main".to_string(),
                owner_class: None,
                code_addr: SnapshotAddr(0x200),
                kind: FunctionKind::RegularFunction,
                is_static: true,
                is_async: false,
                is_generator: false,
                parameter_names: vec![],
                return_type: Some("void".to_string()),
            }),
        );
        pool.insert(SnapshotAddr(0x200), DartObject::Mint(10));
        let funcs = pool.functions();
        assert_eq!(funcs.len(), 1);
    }

    #[test]
    fn test_class_table_empty() {
        let table = class_table::ClassTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_string_table_empty() {
        let table = string_table::StringTable::new();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_dart_object_variants() {
        let objs: Vec<DartObject> = vec![
            DartObject::Mint(42),
            DartObject::Double(3.14),
            DartObject::Bool(true),
            DartObject::Null,
            DartObject::SentinelObject,
        ];
        assert_eq!(objs.len(), 5);
    }
}
