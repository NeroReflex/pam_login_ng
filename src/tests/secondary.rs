#[test]
fn test_autologin() {
    let correct_main = format!("main password <3");
    let intermediate = format!("intermediate_key");
    let autologin = format!("");

    let mut user_cfg = crate::user::UserAuthData::new();
    user_cfg.set_main(&correct_main, &intermediate).unwrap();
    user_cfg.add_secondary_password(&intermediate, &autologin).unwrap();

    let secondary_password = Some(autologin);
    assert_eq!(user_cfg.main_by_auth(&secondary_password).unwrap(), correct_main);
}

#[test]
fn test_secondary() {
    let correct_main = format!("main password <3");
    let intermediate = format!("intermediate_key");
    let secondary_passwords = vec![
        format!("daisujda"),
        format!("sfaffsss")
    ];

    let mut user_cfg = crate::user::UserAuthData::new();
    user_cfg.set_main(&correct_main, &intermediate).unwrap();

    // register every secondary password in the test vector
    for sp in secondary_passwords.iter() {
        user_cfg.add_secondary_password(&intermediate, sp).unwrap();
    }
    
    // attempt to login with each secondary password
    let mut tested: usize = 0;
    for sp in secondary_passwords.iter() {
        let secondary_password = Some(sp.clone());
        assert_eq!(user_cfg.main_by_auth(&secondary_password).unwrap(), correct_main);
        tested += 1;
    }

    assert_eq!(tested, secondary_passwords.len());
}