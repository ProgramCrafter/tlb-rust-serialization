// (c) ProgramCrafter, 2024

use tlb_macro::*;


mod ton {
    use tlb_macro::*;
    
    #[derive(Default)]
    #[tlb_serializable(__fundamental_varuint16)]
    pub struct Coins(u128);
    
    #[derive(Default)]
    #[tlb_serializable(u 4 3bit, workchain, hash_high, hash_low)]
    pub struct Address {
        workchain: u8,
        hash_high: u128,
        hash_low: u128
    }
    
    #[derive(Default)]
    #[tlb_serializable(grams, u 0 1bit)]
    pub struct CurrencyCollection {grams: Coins}
    
    pub trait CellSerialize {
        fn serialize(&self) -> Vec<String>;
    }
    
    // Defining serialization on foreign (std) types.
    impl CellSerialize for u8 {
        fn serialize(&self) -> Vec<String> {  vec![format!("u {self} 8bit")]  }
    }
    impl CellSerialize for u32 {
        fn serialize(&self) -> Vec<String> {  vec![format!("u {self} 32bit")]  }
    }
    impl CellSerialize for u64 {
        fn serialize(&self) -> Vec<String> {  vec![format!("u {self} 64bit")]  }
    }
    impl CellSerialize for u128 {
        fn serialize(&self) -> Vec<String> {  vec![format!("u {self} 128bit")]  }
    }
    impl CellSerialize for bool {
        fn serialize(&self) -> Vec<String> {
            vec![format!("u {} 1bit", if *self {1} else {0})]
        }
    }
}


#[allow(non_camel_case_types)]
#[tlb_enum_serializable]
#[tlb_assert_unsafe(items_prefixes_nonoverlap)]
// #[repr(u16)]
enum CommonMsgInfo {
    #[tlb_item_serializable(u 0 1bit, ihr_disabled, bounce, bounced, src, dest,
                            value, ihr_fee, fwd_fee, created_lt, created_at)]
    int_msg_info {
        ihr_disabled: bool,
        bounce: bool,
        bounced: bool,
        src: ton::Address,
        dest: ton::Address,
        value: ton::CurrencyCollection,
        ihr_fee: ton::Coins,
        fwd_fee: ton::Coins,
        created_lt: u64,
        created_at: u32
    }
}
impl Default for CommonMsgInfo {
    fn default() -> Self {
        CommonMsgInfo::int_msg_info {
            ihr_disabled: true, bounce: true, bounced: false,
            src: Default::default(), dest: Default::default(),
            value: Default::default(), ihr_fee: Default::default(),
            fwd_fee: Default::default(), created_lt: 10001, created_at: 0
        }
    }
}


#[tlb_enum_serializable]
#[repr(u32)]
enum Boc {
    #[tlb_item_serializable(u 0 16bit)] Empty{}  = 0,
    #[tlb_item_serializable()] Normal{} = 0xb5eec792,
}


fn main() {
    use ton::CellSerialize;
    
    println!("{:?}", ton::CurrencyCollection::default().serialize());
    println!("{:?}", CommonMsgInfo::default().serialize());
    println!("{:?}", Boc::Normal{}.serialize());
    println!("{:?}", Boc::Empty{}.serialize());
}
