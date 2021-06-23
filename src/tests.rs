use std::{net::IpAddr, str::FromStr};

use crate::utils::{domain_or_default, ToHostname};

pub const HOSTS_DIR: &str = "testdata/hosts-files";

#[test]
fn test_parse_member_name() {
    use crate::utils::parse_member_name;

    let actual_domains: &mut Vec<Option<&str>> =
        &mut vec!["tld", "domain", "zerotier", "test.subdomain"]
            .iter()
            .map(|s| Some(*s))
            .collect::<Vec<Option<&str>>>();

    actual_domains.push(None); // make sure the None case also gets checked

    for domain in actual_domains {
        let domain_name = domain_or_default(*domain).unwrap().clone();

        assert_eq!(parse_member_name(None, domain_name.clone()), None);

        for name in vec!["islay", "ALL-CAPS", "Capitalized", "with.dots"] {
            assert_eq!(
                parse_member_name(Some(name.to_string()), domain_name.clone()),
                Some(name.to_fqdn(domain_name.clone()).unwrap()),
                "{}",
                name,
            );
        }

        for bad_name in vec![".", "!", "arghle."] {
            assert_eq!(
                parse_member_name(Some(bad_name.to_string()), domain_name.clone()),
                None,
                "{}",
                bad_name,
            );
        }

        for (orig, translated) in vec![("Erik's laptop", "eriks-laptop"), ("!foo", "foo")] {
            assert_eq!(
                parse_member_name(Some(orig.to_string()), domain_name.clone()),
                Some(translated.to_fqdn(domain_name.clone()).unwrap()),
                "{}",
                orig,
            );
        }
    }
}

#[test]
fn test_parse_ip_from_cidr() {
    use crate::utils::parse_ip_from_cidr;

    let results = vec![
        ("192.168.12.1/16", "192.168.12.1"),
        ("10.0.0.0/8", "10.0.0.0"),
        ("fe80::abcd/128", "fe80::abcd"),
    ];

    for (cidr, ip) in results {
        assert_eq!(
            parse_ip_from_cidr(String::from(cidr)),
            IpAddr::from_str(ip).unwrap(),
            "{}",
            cidr
        );
    }
}

#[test]
fn test_domain_or_default() {
    use crate::utils::{domain_or_default, DOMAIN_NAME};
    use std::str::FromStr;
    use trust_dns_server::client::rr::Name;

    assert_eq!(
        domain_or_default(None).unwrap(),
        Name::from_str(DOMAIN_NAME).unwrap()
    );

    assert_eq!(
        domain_or_default(Some("zerotier")).unwrap(),
        Name::from_str("zerotier").unwrap()
    );

    assert_eq!(
        domain_or_default(Some("zerotier.tld")).unwrap(),
        Name::from_str("zerotier.tld").unwrap()
    );

    for bad in vec!["bad.", "~", "!", ".", ""] {
        assert!(domain_or_default(Some(bad)).is_err(), "{}", bad);
    }
}

#[test]
fn test_central_token() {
    use crate::utils::central_token;

    assert!(central_token(None).is_err());
    std::env::set_var("ZEROTIER_CENTRAL_TOKEN", "abcdef");
    assert_eq!(central_token(None).unwrap(), "abcdef");

    let hosts = std::fs::read_to_string("/etc/hosts").unwrap();
    let token = central_token(Some("/etc/hosts"));
    assert!(token.is_ok());
    assert_eq!(token.unwrap(), hosts.trim());
}

#[test]
#[should_panic]
fn test_central_token_panic() {
    use crate::utils::central_token;
    central_token(Some("/nonexistent")).unwrap();
}

#[test]
#[cfg(target_os = "linux")]
fn test_supervise_systemd_green() {
    let table = vec![
        (
            "basic",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                ..Default::default()
            },
        ),
        (
            "with-filled-in-properties",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                domain: Some(String::from("zerotier")),
                authtoken: Some(String::from("/var/lib/zerotier-one/authtoken.secret")),
                hosts_file: Some(String::from("/etc/hosts")),
                wildcard_names: true,
            },
        ),
    ];

    let write = match std::env::var("WRITE_FIXTURES") {
        Ok(var) => var != "",
        Err(_) => false,
    };

    if write {
        eprintln!("Write mode: not testing, but updating unit files")
    }

    for (name, mut props) in table {
        let path = std::path::PathBuf::from(format!("testdata/supervise/systemd/{}.unit", name));

        if !write {
            let path = path.canonicalize();

            assert!(path.is_ok(), "{}", name);
            let expected = std::fs::read_to_string(path.unwrap());
            assert!(expected.is_ok(), "{}", name);
            let testing = props.supervise_template();
            assert!(testing.is_ok(), "{}", name);

            assert_eq!(testing.unwrap(), expected.unwrap(), "{}", name);
        } else {
            assert!(props.validate().is_ok(), "{}", name);

            let template = props.supervise_template();
            assert!(template.is_ok(), "{}", name);
            assert!(
                std::fs::write(path, props.supervise_template().unwrap()).is_ok(),
                "{}",
                name
            );
        }
    }
}

#[test]
#[cfg(target_os = "linux")]
fn test_supervise_systemd_red() {
    let table = vec![
        (
            "bad network",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("123456789101112"),
                token: String::from("/proc/cpuinfo"),
                ..Default::default()
            },
        ),
        (
            "bad token (no file)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("~"),
                ..Default::default()
            },
        ),
        (
            "bad token (dir)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("."),
                ..Default::default()
            },
        ),
        (
            "bad hosts (no file)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                hosts_file: Some(String::from("~")),
                ..Default::default()
            },
        ),
        (
            "bad hosts (dir)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                hosts_file: Some(String::from(".")),
                ..Default::default()
            },
        ),
        (
            "bad authtoken (no file)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                authtoken: Some(String::from("~")),
                ..Default::default()
            },
        ),
        (
            "bad authtoken (dir)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                authtoken: Some(String::from(".")),
                ..Default::default()
            },
        ),
        (
            "bad domain (empty string)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                domain: Some(String::from("")),
                ..Default::default()
            },
        ),
        (
            "bad domain (invalid)",
            crate::supervise::Properties {
                binpath: String::from("zeronsd"),
                network: String::from("1234567891011121"),
                token: String::from("/proc/cpuinfo"),
                domain: Some(String::from("-")),
                ..Default::default()
            },
        ),
    ];

    for (name, mut props) in table {
        assert!(props.validate().is_err(), "{}", name);
    }
}

#[test]
fn test_parse_hosts() {
    use crate::hosts::parse_hosts;
    use std::net::IpAddr;
    use std::str::FromStr;
    use trust_dns_resolver::Name;

    let domain = &Name::from_str("zombocom").unwrap();

    for path in std::fs::read_dir(HOSTS_DIR)
        .unwrap()
        .into_iter()
        .map(|p| p.unwrap())
    {
        if path.metadata().unwrap().is_file() {
            let res = parse_hosts(Some(path.path().display().to_string()), domain.clone());
            assert!(res.is_ok(), "{}", path.path().display());

            let mut table = res.unwrap();

            assert_eq!(
                table
                    .remove(&IpAddr::from_str("127.0.0.1").unwrap())
                    .unwrap()
                    .first()
                    .unwrap(),
                &Name::from_str("localhost").unwrap().append_domain(domain),
                "{}",
                path.path().display(),
            );

            assert_eq!(
                table
                    .remove(&IpAddr::from_str("::1").unwrap())
                    .unwrap()
                    .first()
                    .unwrap(),
                &Name::from_str("localhost").unwrap().append_domain(domain),
                "{}",
                path.path().display(),
            );

            let mut accounted = vec!["islay.localdomain", "islay"]
                .into_iter()
                .map(|s| Name::from_str(s).unwrap().append_domain(domain));

            for name in table
                .remove(&IpAddr::from_str("127.0.1.1").unwrap())
                .unwrap()
            {
                assert!(accounted.any(|s| s.eq(&name)));
            }
        }
    }
}
