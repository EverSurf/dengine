mod testsystem;
use lazy_static::lazy_static;
use serde_json::json;

lazy_static! {
    static ref TS: testsystem::TestSystem = testsystem::TestSystem::new(10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_debot_inner_interfaces() {
    let test = TS.new_test("debot1").await;
    test.deploy().await;
    let res = test.run().await;
    assert_eq!(res.1, vec!["Started".to_owned()]);
}

//#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_debot_invoke_msgs() {
    let test1 = TS.new_test("tda").await;
    let test2 = TS.new_test("tdb").await;
    test2.deploy().await;
    test1.deploy_with_args(json!({"targetAddr": test2.addr.clone()})).await;
    let res = test1.run().await;
    assert_eq!(
        res.1,
        vec![
            format!("Invoking Debot B"),
            format!("DebotB receives question: What is your name?"),
            format!("DebotA receives answer: My name is DebotB"),
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_debot_info() {
    let test = TS.new_test("debot2").await;
    test.deploy().await;
    let res = test.run().await;
    assert_eq!(
        res.1,
        vec![
            format!("Hello, World!"),
            format!("How is it going?"),
            format!("You have entered \"testinput\""),
        ]
    );
}
