mod testsystem;
use lazy_static::lazy_static;

lazy_static!(
    static ref TS: testsystem::TestSystem = testsystem::TestSystem::new(10);
);

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_debot_inner_interfaces() {
    let test = TS.new_test("testDebot3").await;
    //test.deploy().await;
    //test.run().await;
    
    //let steps = serde_json::from_value(json!([])).unwrap();
    //let mut info = build_info(abi, 0, vec![format!("0x8796536366ee21852db56dccb60bc564598b618c865fc50c8b1ab740bba128e3")]);
    //info.name = Some(format!("TestSdk"));
    //info.version = Some(format!("0.4.0"));
    //info.caption = Some(format!("Test for SDK interface"));
    //info.hello = Some(format!("Hello, I'm a test."));
    //TestBrowser::execute_with_details(
    //    client.clone(),
    //    debot_addr.clone(),
    //    keys,
    //    steps,
    //    vec![
    //        format!("test substring1 passed"),
    //        format!("test substring2 passed"),
    //        format!("test mnemonicDeriveSignKeys passed"),
    //        format!("test genRandom passed"),
    //        format!("test naclbox passed"),
    //        format!("test naclKeypairFromSecret passed"),
    //        format!("test hex encode passed"),
    //        format!("test base64 encode passed"),
    //        format!("test mnemonic passed"),
    //        format!("test naclboxopen passed"),
    //        format!("test account passed"),
    //        format!("test hdkeyXprv passed"),
    //        format!("test sign hash passed"),
    //        format!("test hex decode passed"),
    //        format!("test base64 decode passed"),
    //    ],
    //    info,
    //    vec![],
    //).await;
}