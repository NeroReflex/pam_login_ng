#[test]
fn test_autologin() {
    let correct_main = format!("main password <3");
    let intermediate = format!("intermediate_key");
    let autologin = format!("");

    let mut user_cfg = crate::user::User::new();
    user_cfg.set_main(&correct_main, &intermediate).unwrap();
    user_cfg.add_secondary_password(&intermediate, &autologin).unwrap();

    let secondary_password = Some(autologin);
    assert_eq!(user_cfg.main_by_auth(&secondary_password).unwrap(), correct_main);

}