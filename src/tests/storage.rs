#[test]
fn test_main_password_serialization() {
    let first_main = format!("main password <3");
    let intermediate = format!("intermediate_key");

    let provided_password = Some(first_main.clone());

    let dir_name = "test1";

    let source = crate::storage::StorageSource::Path(std::path::PathBuf::from(dir_name));

    {
        let mut user_cfg = crate::user::UserAuthData::new();
        
        // set the main password
        user_cfg.set_main(&first_main, &intermediate).unwrap();

        assert_eq!(user_cfg.main_by_auth(&provided_password).unwrap(), first_main);

        std::fs::create_dir(dir_name).unwrap();
        crate::storage::save_user_auth_data(user_cfg, &source).unwrap();
    }

    match crate::storage::load_user_auth_data(&source) {
        Ok(reloaded) => {
            std::fs::remove_dir(dir_name).unwrap();
            assert_eq!(reloaded.as_ref().unwrap().main_by_auth(&provided_password).unwrap(), first_main)
        },
        Err(error) => {
            std::fs::remove_dir(dir_name).unwrap();
            let error_str = format!("{}", error);
            eprintln!("{}", error_str);
            assert_eq!(1, 2)
        }
    }
}

#[test]
fn test_secondary_password_serialization() {
    let correct_main = format!("main password <3");
    let intermediate = format!("intermediate_key");
    let secondary_passwords = vec![
        format!("daisujda"),
        format!("sfaffsss")
    ];

    let dir_name = "test2";
    let source = crate::storage::StorageSource::Path(std::path::PathBuf::from(dir_name));

    {
        let mut user_cfg = crate::user::UserAuthData::new();
        user_cfg.set_main(&correct_main, &intermediate).unwrap();

        // register every secondary password in the test vector
        for sp in secondary_passwords.iter() {
            user_cfg.add_secondary_password(&intermediate, sp).unwrap();
        }

        std::fs::create_dir(dir_name).unwrap();
        crate::storage::save_user_auth_data(user_cfg, &source).unwrap();
    }

    let mut tested: usize = 0;
    match crate::storage::load_user_auth_data(&source) {
        Ok(reloaded) => {
            std::fs::remove_dir(dir_name).unwrap();
            
            // attempt to login with each secondary password
            
            for sp in secondary_passwords.iter() {
                let secondary_password = Some(sp.clone());
                assert_eq!(reloaded.as_ref().unwrap().main_by_auth(&secondary_password).unwrap(), correct_main);
                tested += 1;
            }
        },
        Err(error) => {
            std::fs::remove_dir(dir_name).unwrap();
            let error_str = format!("{}", error);
            eprintln!("{}", error_str);
        }
    }

    assert_eq!(tested, secondary_passwords.len());
}
