//! The pure functions of the report data (ADR-0010) on a table of cases, straight in SQL.

mod support;

use support::*;

#[tokio::test]
async fn pesel_check_digit_and_date_of_birth() {
    let db = superuser().await;
    for (value, valid) in [
        ("44051401458", true),
        ("02070803628", true),
        ("02270803624", true),  // 2002-07-08: the century in the month
        ("44051401459", false), // wrong check digit
        ("44133101456", false), // month 13
        ("44023001455", false), // 30 February
        ("00022901455", false), // 1900 was not a leap year
        ("00222901451", true),  // 2000 was
        ("4405140145", false),  // 10 digits
        ("4405140145a", false),
        ("", false),
    ] {
        let ok: bool = db
            .query_one("SELECT crm.desk_pesel_valid($1)", &[&value])
            .await
            .unwrap()
            .get(0);
        assert_eq!(ok, valid, "{value}");
    }
    let null: bool = db
        .query_one("SELECT crm.desk_pesel_valid(NULL)", &[])
        .await
        .unwrap()
        .get(0);
    assert!(!null);
}

#[tokio::test]
async fn iban_normalised_or_refused() {
    let db = superuser().await;
    for (value, expected) in [
        (
            "PL61 1090 1014 0000 0712 1981 2874",
            Some("PL61109010140000071219812874"),
        ),
        (
            "pl61109010140000071219812874",
            Some("PL61109010140000071219812874"),
        ),
        (
            "61109010140000071219812874",
            Some("PL61109010140000071219812874"),
        ),
        (
            "DE89\u{00A0}3704 0044 0532 0130 00",
            Some("DE89370400440532013000"),
        ),
        (
            "GB82 WEST 1234 5698 7654 32",
            Some("GB82WEST12345698765432"),
        ),
        ("PL61 1090 1014 0000 0712 1981 2875", None), // check digits
        ("PL61-1090", None),
        ("1234", None),
    ] {
        let got: Option<String> = db
            .query_one("SELECT crm.desk_iban_normalize($1)", &[&value])
            .await
            .unwrap()
            .get(0);
        assert_eq!(got.as_deref(), expected, "{value}");
    }
}

#[tokio::test]
async fn attachment_names_lose_folders_and_control_characters() {
    let db = superuser().await;
    for (raw, name, content_type) in [
        ("C:\\Users\\x\\raport.PDF", "raport.PDF", "application/pdf"),
        ("../../etc/zdjęcie.jpg", "zdjęcie.jpg", "image/jpeg"),
        ("  log\u{0007}.txt ", "log.txt", "text/plain"),
    ] {
        let row = db
            .query_one(
                "SELECT p_name, p_content_type FROM crm.desk_attachment_normalize($1, 10)",
                &[&raw],
            )
            .await
            .unwrap();
        assert_eq!(
            (
                row.get::<_, String>(0).as_str(),
                row.get::<_, String>(1).as_str()
            ),
            (name, content_type)
        );
    }
    let long = format!("{}.png", "a".repeat(300));
    let row = db
        .query_one(
            "SELECT p_name FROM crm.desk_attachment_normalize($1, 10)",
            &[&long],
        )
        .await
        .unwrap();
    let kept: String = row.get(0);
    assert_eq!(kept.chars().count(), 200);
    assert!(kept.ends_with(".png"));
    let too_big = db
        .query_one(
            "SELECT p_name FROM crm.desk_attachment_normalize('a.png', 25 * 1024 * 1024)",
            &[],
        )
        .await
        .unwrap_err();
    let db_error = too_big.as_db_error().unwrap();
    assert_eq!(db_error.code().code(), "U1123");
    assert!(
        db_error.message().contains("(25 MB)"),
        "{}",
        db_error.message()
    );
}

#[tokio::test]
async fn agents_report_periods_weeks_from_monday_months_and_the_previous_ones() {
    let db = superuser().await;
    // today, previous -> week, month, three months as [from, to)
    for (today, previous, expected) in [
        (
            "2026-11-12",
            false,
            [
                "2026-11-09",
                "2026-11-16",
                "2026-11-01",
                "2026-12-01",
                "2026-09-01",
                "2026-12-01",
            ],
        ),
        (
            "2026-11-12",
            true,
            [
                "2026-11-02",
                "2026-11-09",
                "2026-10-01",
                "2026-11-01",
                "2026-06-01",
                "2026-09-01",
            ],
        ),
        (
            "2026-11-09", // a Monday is the first day of its week
            false,
            [
                "2026-11-09",
                "2026-11-16",
                "2026-11-01",
                "2026-12-01",
                "2026-09-01",
                "2026-12-01",
            ],
        ),
        (
            "2026-01-04", // a Sunday at the turn of the year
            true,
            [
                "2025-12-22",
                "2025-12-29",
                "2025-12-01",
                "2026-01-01",
                "2025-08-01",
                "2025-11-01",
            ],
        ),
    ] {
        let row = db
            .query_one(
                "SELECT week_from::text, week_to::text, month_from::text, month_to::text, three_from::text, \
                 three_to::text FROM crm.desk_report_periods($1, $2::text::date)",
                &[&previous, &today],
            )
            .await
            .unwrap();
        let got: Vec<String> = (0..6).map(|i| row.get(i)).collect();
        assert_eq!(got, expected, "{today} previous={previous}");
    }
}
