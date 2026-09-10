use std::env;
use std::fs;
use std::thread;
use std::time::Duration;

use repopact_desktop_api::{
    CreateWorkItemIntent, DesktopService, EditWorkItemIntent, MutationIntent, WorkItemEditsIntent,
};

fn main() {
    let root = env::args()
        .nth(1)
        .expect("usage: desktop-smoke <adopted-repo>");
    let service = DesktopService::new();
    let initial = service
        .open_repository(&root)
        .expect("open adopted repository");
    assert!(initial.validation.valid, "adopted scratch must start valid");
    let baseline_token = initial.snapshot_token.clone();
    let baseline_work = service.list_work_items(None).expect("list work items");
    assert!(
        !baseline_work.is_empty(),
        "adoption should create a work item"
    );
    let work_id = baseline_work[0].id.clone();

    let create_plan = service
        .plan_mutation(MutationIntent::CreateWorkItem(CreateWorkItemIntent {
            title: "Desktop smoke created".to_owned(),
            status: "active".to_owned(),
            date: "2026-09-10".to_owned(),
            owner_scope: "governance".to_owned(),
            affected_scopes: Vec::new(),
            depends_on: Vec::new(),
            provenance: "concrete".to_owned(),
            acceptance_criteria: Vec::new(),
        }))
        .expect("make create plan");
    assert!(create_plan.applicable, "create plan should be applicable");
    let created = service
        .apply_mutation_plan(&create_plan.session_id, &create_plan.plan_handle)
        .expect("apply create plan");
    assert!(created.success, "create plan should apply");
    let created_item = service
        .list_work_items(None)
        .expect("list created item")
        .into_iter()
        .find(|item| item.title == "Desktop smoke created")
        .expect("created item is indexed");

    let created_edit = service
        .plan_mutation(MutationIntent::EditWorkItem(EditWorkItemIntent {
            id: created_item.id.clone(),
            date: "2026-09-10".to_owned(),
            changes: WorkItemEditsIntent {
                title: Some("Desktop smoke edited".to_owned()),
                ..WorkItemEditsIntent::default()
            },
        }))
        .expect("make edit plan");
    let edited = service
        .apply_mutation_plan(&created_edit.session_id, &created_edit.plan_handle)
        .expect("apply edit plan");
    assert!(edited.success, "edit plan should apply");

    let transition = service
        .plan_mutation(MutationIntent::TransitionWorkItem(
            repopact_desktop_api::TransitionWorkItemIntent {
                id: created_item.id.clone(),
                status: "blocked".to_owned(),
            },
        ))
        .expect("make transition plan");
    let transitioned = service
        .apply_mutation_plan(&transition.session_id, &transition.plan_handle)
        .expect("apply transition plan");
    assert!(transitioned.success, "transition plan should apply");

    let intent = MutationIntent::EditWorkItem(EditWorkItemIntent {
        id: work_id.clone(),
        date: "2026-09-10".to_owned(),
        changes: WorkItemEditsIntent {
            title: Some("Scratch desktop edit".to_owned()),
            ..WorkItemEditsIntent::default()
        },
    });
    let stale_plan = service.plan_mutation(intent.clone()).expect("make plan");
    assert!(stale_plan.applicable, "typed edit should be applicable");
    let path = std::path::Path::new(&root).join(&baseline_work[0].path);
    let original = fs::read_to_string(&path).expect("read indexed work item");
    fs::write(&path, format!("{original}\n")).expect("simulate external drift");
    let stale_result = service
        .apply_mutation_plan(&stale_plan.session_id, &stale_plan.plan_handle)
        .expect("stale apply response");
    assert!(
        !stale_result.success && stale_result.stale,
        "stale plan must be rejected"
    );

    let fresh_plan = service.plan_mutation(intent).expect("re-plan after drift");
    let applied = service
        .apply_mutation_plan(&fresh_plan.session_id, &fresh_plan.plan_handle)
        .expect("fresh apply response");
    assert!(applied.success, "fresh typed edit should apply");
    assert!(
        !applied.changed_paths.is_empty(),
        "apply should report changed paths"
    );

    for _ in 0..12 {
        if !service
            .poll_repository_events()
            .expect("poll self event")
            .is_empty()
        {
            break;
        }
        thread::sleep(Duration::from_millis(150));
    }

    let external = service
        .plan_mutation(MutationIntent::CreateWorkItem(CreateWorkItemIntent {
            title: "Watcher probe".to_owned(),
            status: "proposed".to_owned(),
            date: "2026-09-10".to_owned(),
            owner_scope: "governance".to_owned(),
            affected_scopes: Vec::new(),
            depends_on: Vec::new(),
            provenance: "concrete".to_owned(),
            acceptance_criteria: Vec::new(),
        }))
        .expect("make second plan");
    service
        .discard_mutation_plan(&external.session_id, &external.plan_handle)
        .expect("discard plan");
    let current = fs::read_to_string(&path).expect("read current item");
    fs::write(&path, format!("{current}\n")).expect("touch indexed record");
    thread::sleep(Duration::from_millis(300));
    let events = service
        .poll_repository_events()
        .expect("poll external event");
    assert!(
        !events.is_empty(),
        "watcher should emit an external refresh event"
    );
    assert!(events
        .iter()
        .any(|event| event.origin == repopact_desktop_api::ChangeOrigin::External));

    let old_session_plan = service
        .plan_mutation(MutationIntent::TransitionWorkItem(
            repopact_desktop_api::TransitionWorkItemIntent {
                id: work_id,
                status: "completed".to_owned(),
            },
        ))
        .expect("make session-bound plan");
    let switched = service.open_repository(env::current_dir().expect("current directory"));
    assert!(switched.is_ok(), "switch to a local repository should work");
    assert!(service
        .apply_mutation_plan(&old_session_plan.session_id, &old_session_plan.plan_handle)
        .is_err());

    println!("desktop smoke: valid snapshot {baseline_token}, stale-plan rejection, apply, watcher refresh, and stale-session rejection passed");
}
