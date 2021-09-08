use std::convert::TryInto;

use crate::*;
use hex::decode;

fn str_to_h256(src: &str) -> H256 {
    let src = decode(src).unwrap();
    assert!(src.len() == 32);
    let data: [u8; 32] = src.try_into().unwrap();
    H256::from(data)
}

fn str_to_vec(src: &str) -> Vec<u8> {
    decode(src).unwrap()
}

#[test]
fn test_ckb_smt_verify1() {
    let key = str_to_h256("381dc5391dab099da5e28acd1ad859a051cf18ace804d037f12819c6fbc0e18b");
    let val = str_to_h256("9158ce9b0e11dd150ba2ae5d55c1db04b1c5986ec626f2e38a93fe8ad0b2923b");
    let root_hash = str_to_h256("ebe0fab376cd802d364eeb44af20c67a74d6183a33928fead163120ef12e6e06");
    let proof = str_to_vec(
        "4c4fff51ff322de8a89fe589987f97220cfcb6820bd798b31a0b56ffea221093d35f909e580b00000000000000000000000000000000000000000000000000000000000000");

    let mut builder = CkbSmtBuilder::new();
    builder.insert(&key, &val);

    let smt = builder.build().unwrap();
    assert!(smt.verify(&root_hash, &proof).is_ok());
}

#[test]
fn test_ckb_smt_verify2() {
    let key = str_to_h256("a9bb945be71f0bd2757d33d2465b6387383da42f321072e47472f0c9c7428a8a");
    let val = str_to_h256("a939a47335f777eac4c40fbc0970e25f832a24e1d55adc45a7b76d63fe364e82");
    let root_hash = str_to_h256("6e5c722644cd55cef8c4ed886cd8b44027ae9ed129e70a4b67d87be1c6857842");
    let proof = str_to_vec(
        "4c4fff51fa8aaa2aece17b92ec3f202a40a09f7286522bae1e5581a2a49195ab6781b1b8090000000000000000000000000000000000000000000000000000000000000000");

    let mut builder = CkbSmtBuilder::new();
    builder.insert(&key, &val);

    let smt = builder.build().unwrap();
    assert!(smt.verify(&root_hash, &proof).is_ok());
}

#[test]
fn test_ckb_smt_verify3() {
    let key = str_to_h256("e8c0265680a02b680b6cbc880348f062b825b28e237da7169aded4bcac0a04e5");
    let val = str_to_h256("2ca41595841e46ce8e74ad749e5c3f1d17202150f99c3d8631233ebdd19b19eb");
    let root_hash = str_to_h256("c8f513901e34383bcec57c368628ce66da7496df0a180ee1e021df3d97cb8f7b");
    let proof = str_to_vec(
        "4c4fff51fa8aaa2aece17b92ec3f202a40a09f7286522bae1e5581a2a49195ab6781b1b8090000000000000000000000000000000000000000000000000000000000000000");

    let mut builder = CkbSmtBuilder::new();
    builder.insert(&key, &val);

    let smt = builder.build().unwrap();
    assert!(smt.verify(&root_hash, &proof).is_ok());
}

#[test]
fn test_ckb_smt_verify_invalid() {
    let key = str_to_h256("e8c0265680a02b680b6cbc880348f062b825b28e237da7169aded4bcac0a04e5");
    let val = str_to_h256("2ca41595841e46ce8e74ad749e5c3f1d17202150f99c3d8631233ebdd19b19eb");
    let root_hash = str_to_h256("a4cbf1b69a848396ac759f362679e2b185ac87a17cba747d2db1ef6fd929042f");
    let proof =
        str_to_vec("4c50fe32845309d34f132cd6f7ac6a7881962401adc35c19a18d4fffeb511b97eabf86");

    let mut builder = CkbSmtBuilder::new();
    builder.insert(&key, &val);

    let smt = builder.build().unwrap();
    assert!(!smt.verify(&root_hash, &proof).is_ok());
}
