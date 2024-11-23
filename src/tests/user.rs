#[test]
fn test_main_password_change() {
    let first_main = format!("main password <3");
    let second_main = format!("2nd main password :B");
    let intermediate = format!("intermediate_key");

    let mut user_cfg = crate::user::UserAuthData::new();

    // set the main password
    user_cfg.set_main(&first_main, &intermediate).unwrap();

    // change the main password
    user_cfg.set_main(&second_main, &intermediate).unwrap();
}

#[test]
fn test_main_password_auth() {
    let first_main = format!("main password <3");
    let intermediate = format!("intermediate_key");

    let mut user_cfg = crate::user::UserAuthData::new();

    // set the main password
    user_cfg.set_main(&first_main, &intermediate).unwrap();

    let provided_password = Some(first_main.clone());
    assert_eq!(
        user_cfg.main_by_auth(&provided_password).unwrap(),
        first_main
    );
}
