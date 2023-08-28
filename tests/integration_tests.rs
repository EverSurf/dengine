mod testsystem;
use lazy_static::lazy_static;

lazy_static!(
    static ref TS: testsystem::TestSystem = testsystem::TestSystem::new(10);
);

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_debot_inner_interfaces() {
    let test = TS.new_test("debot1").await;
    test.deploy().await;
    let res = test.run().await;
    assert_eq!(res.1, vec!["Started".to_owned()]);
    
}