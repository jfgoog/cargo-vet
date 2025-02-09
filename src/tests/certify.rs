use super::*;
use std::fmt::Write;

#[test]
fn mock_simple_suggested_criteria() {
    let mock = MockMetadata::simple();

    let metadata = mock.metadata();

    let (mut config, mut audits, imports) = files_no_exemptions(&metadata);

    config.policy.insert(
        "first-party".to_string(),
        dep_policy([("third-party1", ["strong-reviewed"])]),
    );

    audits.audits.insert(
        "third-party1".to_owned(),
        vec![
            full_audit(ver(2), "weak-reviewed"),
            full_audit(ver(3), "reviewed"),
            full_audit(ver(4), "strong-reviewed"),
            delta_audit(ver(6), ver(DEFAULT_VER), "strong-reviewed"),
            delta_audit(ver(7), ver(DEFAULT_VER), "reviewed"),
            delta_audit(ver(8), ver(DEFAULT_VER), "weak-reviewed"),
        ],
    );
    audits.audits.insert(
        "third-party2".to_owned(),
        vec![
            full_audit(ver(2), "weak-reviewed"),
            full_audit(ver(3), "reviewed"),
            full_audit(ver(4), "strong-reviewed"),
            delta_audit(ver(6), ver(DEFAULT_VER), "strong-reviewed"),
            delta_audit(ver(7), ver(DEFAULT_VER), "reviewed"),
            delta_audit(ver(8), ver(DEFAULT_VER), "weak-reviewed"),
        ],
    );

    let store = Store::mock(config, audits, imports);
    let report = crate::resolver::resolve(&metadata, None, &store);

    let mut output = String::new();
    for (from, to, descr) in [
        (None, DEFAULT_VER, "full audit"),
        // from existing audit
        (Some(2), DEFAULT_VER, "from weak-reviewed"),
        (Some(3), DEFAULT_VER, "from reviewed"),
        (Some(4), DEFAULT_VER, "from strong-reviewed"),
        // to existing audit
        (None, 6, "to strong-reviewed"),
        (None, 7, "to reviewed"),
        (None, 8, "to weak-reviewed"),
        // bridge existing audits
        (Some(2), 6, "from weak-reviewed to strong-reviewed"),
        (Some(2), 7, "from weak-reviewed to reviewed"),
        (Some(2), 8, "from weak-reviewed to weak-reviewed"),
        (Some(3), 6, "from reviewed to strong-reviewed"),
        (Some(3), 7, "from reviewed to reviewed"),
        (Some(3), 8, "from reviewed to weak-reviewed"),
        (Some(4), 6, "from strong-reviewed to strong-reviewed"),
        (Some(4), 7, "from strong-reviewed to reviewed"),
        (Some(4), 8, "from strong-reviewed to weak-reviewed"),
    ] {
        let from = from.map(ver);
        let to = ver(to);
        writeln!(
            output,
            "{} ({} -> {})",
            descr,
            from.as_ref().map_or("root".to_owned(), |v| v.to_string()),
            to
        )
        .unwrap();
        writeln!(
            output,
            "  third-party1: {:?}",
            report.compute_suggested_criteria("third-party1", from.as_ref(), &to)
        )
        .unwrap();
        writeln!(
            output,
            "  third-party2: {:?}",
            report.compute_suggested_criteria("third-party2", from.as_ref(), &to)
        )
        .unwrap();
    }

    insta::assert_snapshot!("mock-simple-suggested-criteria", output);
}

#[test]
fn mock_simple_certify_flow() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let mut store = Store::mock(config, audits, imports);

    let output = BasicTestOutput::with_callbacks(
        |_| Ok("\n".to_owned()),
        |_| {
            Ok("\
            I, testing, certify that I have audited version 10.0.0 of third-party1 in accordance with the above criteria.\n\
            \n\
            These are testing notes. They contain some\n\
            newlines. Trailing whitespace        \n    \
            and leading whitespace\n\
            \n".to_owned())
        },
    );

    let cfg = mock_cfg_args(
        &metadata,
        [
            "cargo",
            "vet",
            "certify",
            "third-party1",
            "10.0.0",
            "--who",
            "testing",
        ],
    );
    let sub_args = if let Some(crate::cli::Commands::Certify(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    crate::do_cmd_certify(
        &output.clone().as_dyn(),
        &cfg,
        sub_args,
        &mut store,
        None,
        None,
    )
    .expect("do_cmd_certify failed");

    let audits = crate::serialization::to_formatted_toml(&store.audits, None).unwrap();

    let result = format!("OUTPUT:\n{output}\nAUDITS:\n{audits}");

    insta::assert_snapshot!("mock-simple-certify-flow", result);
}

#[test]
fn mock_delta_certify_flow() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let mut store = Store::mock(config, audits, imports);

    let output = BasicTestOutput::with_callbacks(
        |_| Ok("\n".to_owned()),
        |_| {
            Ok("\
            I, testing, certify that I have audited the changes from version 10.0.0 to 10.0.1 of third-party1 in accordance with the above criteria.\n\
            \n\
            These are testing notes. They contain some\n\
            newlines. Trailing whitespace        \n    \
            and leading whitespace\n\
            \n".to_owned())
        },
    );

    let cfg = mock_cfg_args(
        &metadata,
        [
            "cargo",
            "vet",
            "certify",
            "third-party1",
            "10.0.0",
            "10.0.1",
            "--who",
            "testing",
            "--criteria",
            "safe-to-deploy",
        ],
    );
    let sub_args = if let Some(crate::cli::Commands::Certify(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    crate::do_cmd_certify(
        &output.clone().as_dyn(),
        &cfg,
        sub_args,
        &mut store,
        None,
        None,
    )
    .expect("do_cmd_certify failed");

    let audits = crate::serialization::to_formatted_toml(&store.audits, None).unwrap();

    let result = format!("OUTPUT:\n{output}\nAUDITS:\n{audits}");

    insta::assert_snapshot!("mock-delta-certify-flow", result);
}

#[test]
fn mock_wildcard_certify_flow() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let output = BasicTestOutput::with_callbacks(
        |_| Ok("\n".to_owned()),
        |_| {
            Ok("\
            I, testing, certify that any version of third-party1 published by 'testuser' between 2022-12-12 and 2024-01-01 will satisfy the above criteria.\n\
            \n\
            These are testing notes. They contain some\n\
            newlines. Trailing whitespace        \n    \
            and leading whitespace\n\
            \n".to_owned())
        },
    );

    let cfg = mock_cfg_args(
        &metadata,
        [
            "cargo",
            "vet",
            "certify",
            "third-party1",
            "--wildcard",
            "testuser",
            "--who",
            "testing",
        ],
    );
    let sub_args = if let Some(crate::cli::Commands::Certify(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    let mut network = Network::new_mock();

    MockRegistryBuilder::new()
        .user(2, "testuser", "Test user")
        .user(5, "otheruser", "Other User")
        .package(
            "third-party1",
            &[
                reg_published_by(ver(9), Some(5), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .serve(&mut network);

    let mut store = Store::mock_online(&cfg, config, audits, imports, &network, true)
        .expect("store acquisition failed");

    crate::do_cmd_certify(
        &output.clone().as_dyn(),
        &cfg,
        sub_args,
        &mut store,
        Some(&network),
        None,
    )
    .expect("do_cmd_certify failed");

    let audits = crate::serialization::to_formatted_toml(
        &store.audits,
        Some(&crate::storage::user_info_map(&store.imports)),
    )
    .unwrap();

    let result = format!("OUTPUT:\n{output}\nAUDITS:\n{audits}");

    insta::assert_snapshot!(result);
}

#[test]
fn mock_trust_flow_simple() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let output = BasicTestOutput::with_callbacks(|_| Ok("\n".to_owned()), |_| unimplemented!());

    let cfg = mock_cfg_args(&metadata, ["cargo", "vet", "trust", "third-party1"]);
    let sub_args = if let Some(crate::cli::Commands::Trust(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    let mut network = Network::new_mock();
    MockRegistryBuilder::new()
        .user(2, "testuser", "Test user")
        .package(
            "third-party1",
            &[
                reg_published_by(ver(1), None, "2022-10-12"),
                reg_published_by(ver(9), Some(2), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .serve(&mut network);

    let mut store = Store::mock_online(&cfg, config, audits, imports, &network, true)
        .expect("store acquisition failed");

    crate::do_cmd_trust(
        &output.clone().as_dyn(),
        &cfg,
        sub_args,
        &mut store,
        Some(&network),
    )
    .expect("do_cmd_trust failed");

    let audits = crate::serialization::to_formatted_toml(
        &store.audits,
        Some(&crate::storage::user_info_map(&store.imports)),
    )
    .unwrap();

    let result = format!("OUTPUT:\n{output}\nAUDITS:\n{audits}");

    insta::assert_snapshot!(result);
}

#[test]
fn mock_trust_flow_ambiguous() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let output = BasicTestOutput::with_callbacks(|_| Ok("\n".to_owned()), |_| unimplemented!());

    let cfg = mock_cfg_args(&metadata, ["cargo", "vet", "trust", "third-party1"]);
    let sub_args = if let Some(crate::cli::Commands::Trust(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    let mut network = Network::new_mock();
    MockRegistryBuilder::new()
        .user(2, "testuser", "Test user")
        .user(5, "otheruser", "Other user")
        .package(
            "third-party1",
            &[
                reg_published_by(ver(1), None, "2022-10-12"),
                reg_published_by(ver(9), Some(5), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .serve(&mut network);

    let mut store = Store::mock_online(&cfg, config, audits, imports, &network, true)
        .expect("store acquisition failed");

    let error = crate::do_cmd_trust(&output.as_dyn(), &cfg, sub_args, &mut store, Some(&network))
        .expect_err("do_cmd_trust succeeded");

    insta::assert_snapshot!(format!("{error:?}"));
}

#[test]
fn mock_trust_flow_explicit() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let output = BasicTestOutput::with_callbacks(|_| Ok("\n".to_owned()), |_| unimplemented!());

    let cfg = mock_cfg_args(
        &metadata,
        ["cargo", "vet", "trust", "third-party1", "testuser"],
    );
    let sub_args = if let Some(crate::cli::Commands::Trust(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    let mut network = Network::new_mock();
    MockRegistryBuilder::new()
        .user(2, "testuser", "Test user")
        .user(5, "otheruser", "Other user")
        .package(
            "third-party1",
            &[
                reg_published_by(ver(1), None, "2022-10-12"),
                reg_published_by(ver(9), Some(5), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .serve(&mut network);

    let mut store = Store::mock_online(&cfg, config, audits, imports, &network, true)
        .expect("store acquisition failed");

    crate::do_cmd_trust(
        &output.clone().as_dyn(),
        &cfg,
        sub_args,
        &mut store,
        Some(&network),
    )
    .expect("do_cmd_trust failed");

    let audits = crate::serialization::to_formatted_toml(
        &store.audits,
        Some(&crate::storage::user_info_map(&store.imports)),
    )
    .unwrap();

    let result = format!("OUTPUT:\n{output}\nAUDITS:\n{audits}");

    insta::assert_snapshot!(result);
}

#[test]
fn mock_trust_flow_all() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let output = BasicTestOutput::with_callbacks(|_| Ok("\n".to_owned()), |_| unimplemented!());

    let cfg = mock_cfg_args(&metadata, ["cargo", "vet", "trust", "--all", "testuser"]);
    let sub_args = if let Some(crate::cli::Commands::Trust(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    let mut network = Network::new_mock();
    MockRegistryBuilder::new()
        .user(2, "testuser", "Test user")
        .user(5, "otheruser", "Other user")
        .package(
            "third-party1",
            &[
                reg_published_by(ver(1), None, "2022-10-12"),
                reg_published_by(ver(9), Some(5), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .package(
            "transitive-third-party1",
            &[
                reg_published_by(ver(1), None, "2022-10-12"),
                reg_published_by(ver(9), Some(2), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .package(
            "third-party2",
            &[reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12")],
        )
        .serve(&mut network);

    let mut store = Store::mock_online(&cfg, config, audits, imports, &network, true)
        .expect("store acquisition failed");

    crate::do_cmd_trust(
        &output.clone().as_dyn(),
        &cfg,
        sub_args,
        &mut store,
        Some(&network),
    )
    .expect("do_cmd_trust failed");

    let audits = crate::serialization::to_formatted_toml(
        &store.audits,
        Some(&crate::storage::user_info_map(&store.imports)),
    )
    .unwrap();

    let result = format!("OUTPUT:\n{output}\nAUDITS:\n{audits}");

    insta::assert_snapshot!(result);
}

#[test]
fn mock_trust_flow_all_allow_multiple() {
    let mock = MockMetadata::simple();

    let _enter = TEST_RUNTIME.enter();
    let metadata = mock.metadata();

    let (config, audits, imports) = files_inited(&metadata);

    let output = BasicTestOutput::with_callbacks(|_| Ok("\n".to_owned()), |_| unimplemented!());

    let cfg = mock_cfg_args(
        &metadata,
        [
            "cargo",
            "vet",
            "trust",
            "--all",
            "testuser",
            "--allow-multiple-publishers",
        ],
    );
    let sub_args = if let Some(crate::cli::Commands::Trust(sub_args)) = &cfg.cli.command {
        sub_args
    } else {
        unreachable!();
    };

    let mut network = Network::new_mock();
    MockRegistryBuilder::new()
        .user(2, "testuser", "Test user")
        .user(5, "otheruser", "Other user")
        .package(
            "third-party1",
            &[
                reg_published_by(ver(1), None, "2022-10-12"),
                reg_published_by(ver(9), Some(5), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .package(
            "transitive-third-party1",
            &[
                reg_published_by(ver(1), None, "2022-10-12"),
                reg_published_by(ver(9), Some(2), "2022-10-12"),
                reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12"),
            ],
        )
        .package(
            "third-party2",
            &[reg_published_by(ver(DEFAULT_VER), Some(2), "2022-12-12")],
        )
        .serve(&mut network);

    let mut store = Store::mock_online(&cfg, config, audits, imports, &network, true)
        .expect("store acquisition failed");

    crate::do_cmd_trust(
        &output.clone().as_dyn(),
        &cfg,
        sub_args,
        &mut store,
        Some(&network),
    )
    .expect("do_cmd_trust failed");

    let audits = crate::serialization::to_formatted_toml(
        &store.audits,
        Some(&crate::storage::user_info_map(&store.imports)),
    )
    .unwrap();

    let result = format!("OUTPUT:\n{output}\nAUDITS:\n{audits}");

    insta::assert_snapshot!(result);
}
