use super::{classify_row, OutputDomain, RowClassifier};
use automexia_extension_api::SemanticSeverity::{Error, Info, Success, Warning};

#[test]
fn kubernetes_resource_table_matrix_uses_status_fields_not_names() {
    for (header, row, severity) in [
        ("NAME  READY  UP-TO-DATE  AVAILABLE  AGE", "error  3/3  3  3  1d", Some(Success)),
        ("NAME  READY  UP-TO-DATE  AVAILABLE  AGE", "api  3/3  2  3  1d", Some(Warning)),
        ("NAME  READY  UP-TO-DATE  AVAILABLE  AGE", "api  0/0  0  0  1d", Some(Info)),
        ("NAME  READY  AGE", "db  2/3  1d", Some(Warning)),
        ("NAMESPACE  NAME  DESIRED  CURRENT  READY  UP-TO-DATE  AVAILABLE  NODE SELECTOR  AGE", "demo  daemon  3  3  2  3  2  <none>  1d", Some(Warning)),
        ("NAME  STATUS  COMPLETIONS  DURATION  AGE", "warning  Complete  1/1  1m  1d", Some(Info)),
        ("NAME  STATUS  COMPLETIONS  DURATION  AGE", "job  FailureTarget  0/1  1m  1m", Some(Error)),
        ("NAME  COMPLETIONS  DURATION  AGE", "job  0/1  1m  1m", Some(Warning)),
        ("NAME  SCHEDULE  TIMEZONE  SUSPEND  ACTIVE  LAST SCHEDULE  AGE", "backup  * * * * *  <none>  True  0  3m  1d", Some(Warning)),
        ("NAME  STATUS  ROLES  AGE  VERSION", "node  Ready,SchedulingDisabled  worker  1d  v1.36.0", Some(Warning)),
        ("NAME  CAPACITY  ACCESS MODES  RECLAIM POLICY  STATUS  CLAIM  STORAGECLASS  AGE", "disk  1Gi  RWO  Delete  Released  demo/data  standard  1d", Some(Warning)),
        ("NAME  STATUS  VOLUME  CAPACITY  ACCESS MODES  STORAGECLASS  AGE", "error  Bound  disk  1Gi  RWO  standard  1d", Some(Success)),
        ("NAME  STATUS  AGE", "demo  Active  1d", Some(Success)),
        ("NAME  TYPE  CLUSTER-IP  EXTERNAL-IP  PORT(S)  AGE", "api  LoadBalancer  10.0.0.1  <pending>  80/TCP  1d", Some(Warning)),
        ("LAST SEEN  TYPE  REASON  OBJECT  MESSAGE", "2m  Warning  FailedMount  pod/api  could not mount volume", Some(Warning)),
        ("NAME  REFERENCE  TARGETS  MINPODS  MAXPODS  REPLICAS  AGE", "api  Deployment/api  cpu: <unknown>/80%  1  5  1  1d", Some(Warning)),
        ("NAME  AGE  SIGNERNAME  REQUESTOR  CONDITION", "csr  1d  example.test/signer  system:node:fixture  Approved,Issued", Some(Success)),
        ("NAME  AGE  SIGNERNAME  REQUESTOR  CONDITION", "csr  1d  example.test/signer  system:node:fixture  Approved,Denied", Some(Error)),
        ("NAME  SERVICE  AVAILABLE  AGE", "v1.metrics.example.test  demo/metrics  False  1d", Some(Error)),
        ("NAME  SERVICE  AVAILABLE  AGE", "v1.metrics.example.test  demo/metrics  False (MissingEndpoints)  1d", Some(Error)),
        ("NAME  SERVICE  AVAILABLE  AGE", "v1.metrics.example.test  demo/metrics  Unknown (ProbePending)  1d", Some(Warning)),
        ("NAME  CLASS  HOSTS  ADDRESS  PORTS  AGE", "ingress  example  app.example.test  192.0.2.1  80  1d", Some(Info)),
        ("NAME  READY  UP-TO-DATE  AVAILABLE  AGE", "error  4294967296/1  1  1  1d", None),
        ("NAME  STATUS  COMPLETIONS  DURATION  AGE", "error  FutureState  0/1  1m  1m", None),
    ] {
        let mut classifier = RowClassifier::default();
        classifier.classify(header);
        let actual = classifier.classify(row).expect(row);
        assert_eq!(actual.domain, OutputDomain::Kubernetes, "{row}");
        assert_eq!(actual.severity, severity, "{row}");
        let mut pipes = RowClassifier::default();
        pipes.classify(&format!("| {} |", header.replace("  ", " | ")));
        assert_eq!(pipes.classify(&format!("| {} |", row.replace("  ", " | "))), Some(actual));
    }
}

#[test]
fn kubernetes_schema_is_bounded_and_does_not_leak_into_other_output() {
    let mut classifier = RowClassifier::default();
    classifier.classify("NAME  READY  AGE");
    assert_eq!(
        classifier.classify("db  1/2  1d").unwrap().domain,
        OutputDomain::Kubernetes
    );
    classifier.reset();
    assert_eq!(
        classifier.classify("[ERROR] actual log").unwrap().domain,
        OutputDomain::General
    );
    classifier.classify("NAME  READY  AGE");
    assert_eq!(classifier.classify(""), None);
    assert_eq!(classifier.classify("TASK  STATUS"), None);
    assert_eq!(
        classifier
            .classify("build  warning: retrying")
            .unwrap()
            .domain,
        OutputDomain::General
    );
    classifier.classify("NAME  STATUS  AGE");
    assert_eq!(
        classifier
            .classify("build  warning: retrying  1d")
            .unwrap()
            .domain,
        OutputDomain::General
    );
    classifier.classify("NAME READY  AGE");
    assert_eq!(
        classifier.classify(&"x".repeat(super::MAX_ROW_BYTES + 1)),
        None
    );
    assert_eq!(classifier.classify(&"NAME  ".repeat(100)), None);
}

#[test]
fn pending_storage_keeps_empty_columns_and_its_kubernetes_owner() {
    let pending = format!(
        "{:<6}{:<9}{:<8}{:<10}{:<14}{:<14}{}",
        "data", "Pending", "", "", "", "standard", "1m"
    );
    for (header, row) in [
        (
            "NAME  STATUS   VOLUME  CAPACITY  ACCESS MODES  STORAGECLASS  AGE",
            pending.as_str(),
        ),
        (
            "| NAME | STATUS | VOLUME | CAPACITY | ACCESS MODES | STORAGECLASS | AGE |",
            "| data | Pending | | | | standard | 1m |",
        ),
        (
            "NAME\tSTATUS\tVOLUME\tCAPACITY\tACCESS MODES\tSTORAGECLASS\tAGE",
            "data\tPending\t\t\t\tstandard\t1m",
        ),
    ] {
        let mut classifier = RowClassifier::default();
        classifier.classify(header);
        let actual = classifier.classify(row).unwrap();
        assert_eq!(actual.domain, OutputDomain::Kubernetes);
        assert_eq!(actual.severity, Some(Warning));
    }
}

#[test]
fn kind_prefixed_resources_work_without_a_visible_header() {
    for (row, severity) in [
        ("deployment.apps/api  2/3  3  2  1d", Warning),
        ("demo  statefulset.apps/db  2/2  1d", Success),
        ("job.batch/backup  Complete  1/1  1m  1d", Info),
        ("namespace/demo  Terminating  1d", Warning),
    ] {
        let actual = classify_row(row).unwrap();
        assert_eq!(actual.domain, OutputDomain::Kubernetes, "{row}");
        assert_eq!(actual.severity, Some(severity), "{row}");
    }
    assert_eq!(classify_row("path/api  2/3  3  2  1d"), None);
}
