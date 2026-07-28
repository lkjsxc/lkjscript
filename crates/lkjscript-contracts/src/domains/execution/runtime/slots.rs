use crate::{ContractFact, ContractItem, ContractItemKind};

pub(super) fn runtime_slots() -> ContractItem {
    let facts = [
        (
            "identity-i64",
            "IdentityI64",
            "(state,i64)->i64 pure test boundary",
        ),
        (
            "poll",
            "Poll",
            "(state)->deadline and fuel status noncollecting",
        ),
        (
            "enter-function",
            "EnterFunction",
            "(state,function)->status",
        ),
        (
            "standard-input",
            "StdinHandle",
            "(island-state,capability/stdio)->resource/input-stream",
        ),
        (
            "byte-vector-new",
            "ByteVectorNew",
            "(island-state,i64)->unique/byte-vector",
        ),
        (
            "byte-vector-move",
            "ByteVectorMove",
            "(island-state,unique/byte-vector)->unique/byte-vector",
        ),
        (
            "byte-vector-borrow-shared",
            "ByteVectorBorrowShared",
            "(island-state,unique/byte-vector)->loan/byte-slice",
        ),
        (
            "byte-vector-borrow-exclusive",
            "ByteVectorBorrowExclusive",
            "(island-state,unique/byte-vector)->loan/byte-slice-mut",
        ),
        (
            "byte-slice-length",
            "ByteSliceLength",
            "(island-state,loan/byte-slice)->i64",
        ),
        (
            "byte-slice-byte-at",
            "ByteSliceByteAt",
            "(island-state,loan/byte-slice,i64)->i64",
        ),
        (
            "byte-slice-mut-set-byte",
            "ByteSliceMutSetByte",
            "(island-state,loan/byte-slice-mut,i64,i64)->unit",
        ),
        (
            "byte-slice-end",
            "ByteSliceEnd",
            "(island-state,loan/byte-slice)->unit",
        ),
        (
            "byte-slice-mut-end",
            "ByteSliceMutEnd",
            "(island-state,loan/byte-slice-mut)->unit",
        ),
        (
            "byte-vector-drop",
            "ByteVectorDrop",
            "(island-state,unique/byte-vector)->unit",
        ),
        (
            "collect-reference",
            "CollectReference",
            "(state,reference)->status",
        ),
        (
            "heap-dispatch",
            "HeapDispatch",
            "(state,operation,args)->status",
        ),
        ("reserve-frame", "ReserveFrame", "(state,slots)->status"),
        ("register-frame", "RegisterFrame", "(state,frame)->status"),
        (
            "publish-safepoint",
            "PublishSafepoint",
            "(state,map)->status",
        ),
        (
            "unregister-frame",
            "UnregisterFrame",
            "(state,frame)->status",
        ),
    ];
    facts.into_iter().fold(
        ContractItem::new("slots", ContractItemKind::Operation).semantic_order(),
        |item, (id, name, value)| item.fact(ContractFact::required(id, name, value)),
    )
}
