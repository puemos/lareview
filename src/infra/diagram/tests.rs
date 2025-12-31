use crate::infra::diagram::*;
use serde_json::json;

fn parse_sequence_json(value: serde_json::Value) -> SequenceDiagram {
    match parse_json(&value.to_string()).expect("JSON produces diagram") {
        Diagram::Sequence(seq) => seq,
        other => panic!("Expected Sequence diagram, got {:?}", other),
    }
}

fn parse_flow_json(value: serde_json::Value) -> FlowDiagram {
    match parse_json(&value.to_string()).expect("JSON produces diagram") {
        Diagram::Flow(flow) => flow,
        other => panic!("Expected Flow diagram, got {:?}", other),
    }
}

fn render_sequence_outputs(seq: &SequenceDiagram) -> (String, String) {
    let d2 = D2Renderer
        .render(&Diagram::Sequence(seq.clone()))
        .expect("D2 render succeeds");
    let mermaid = MermaidRenderer
        .render(&Diagram::Sequence(seq.clone()))
        .expect("Mermaid render succeeds");
    (d2, mermaid)
}

fn count_lines_starting_with(text: &str, prefix: &str) -> usize {
    text.lines()
        .filter(|line| line.trim_start().starts_with(prefix))
        .count()
}

fn assert_contains_all(text: &str, parts: &[&str]) {
    for part in parts {
        assert!(
            text.contains(part),
            "Expected output to contain '{part}', got:\n{text}"
        );
    }
}

#[test]
fn flow_builder_happy_path() {
    let flow = FlowDiagram::builder()
        .title("E-Commerce Architecture")
        .direction(Direction::LeftToRight)
        .node("client", "Web Client", NodeKind::User)
        .node("api", "API", NodeKind::Service)
        .node("db", "DB", NodeKind::Database)
        .edge("client", "api", "HTTPS")
        .edge("api", "db", "SQL")
        .build()
        .expect("valid diagram");

    let d2 = D2Renderer.render(&Diagram::Flow(flow.clone())).unwrap();
    let mermaid = MermaidRenderer.render(&Diagram::Flow(flow)).unwrap();

    assert!(d2.contains("client -> api"));
    assert!(mermaid.contains("flowchart LR"));
}

#[test]
fn parse_json_auto_heals_prefixed_payload() {
    let json = "{\"type\":\"flow\",\"data\":{\"direction\":\"LR\",\"nodes\":[{\"id\":\"a\",\"label\":\"A\",\"kind\":\"generic\"}]}}";
    let payload = format!("diagram{json} diff_refs[]");
    let diagram = parse_json(&payload).expect("auto-heal");
    match diagram {
        Diagram::Flow(flow) => assert_eq!(flow.nodes.len(), 1),
        other => panic!("Expected Flow diagram, got {:?}", other),
    }
}

#[test]
fn parse_json_auto_heals_common_json_mistakes() {
    let trailing = "{\"type\":\"flow\",\"data\":{\"direction\":\"LR\",\"nodes\":[{\"id\":\"a\",\"label\":\"A\",\"kind\":\"generic\"},],}}";
    let diagram = parse_json(trailing).expect("auto-heal trailing commas");
    match diagram {
        Diagram::Flow(flow) => assert_eq!(flow.nodes.len(), 1),
        other => panic!("Expected Flow diagram, got {:?}", other),
    }

    let single_quotes = "{'type':'flow','data':{'direction':'LR','nodes':[{'id':'a','label':'A','kind':'generic'}]}}";
    let diagram = parse_json(single_quotes).expect("auto-heal single quotes");
    match diagram {
        Diagram::Flow(flow) => assert_eq!(flow.nodes.len(), 1),
        other => panic!("Expected Flow diagram, got {:?}", other),
    }
}

#[test]
fn parse_json_auto_heals_missing_type_sequence_from_log() {
    let data_only = r#"{
  "actors": [
    { "id": "user", "label": "User", "kind": "user" },
    { "id": "preview", "label": "PlaylistPreview", "kind": "service" },
    { "id": "video", "label": "Video Element", "kind": "generic" },
    { "id": "hlsjs", "label": "HLS.js", "kind": "service" },
    { "id": "playlist", "label": "Playlist URI", "kind": "external" }
  ],
  "messages": [
    { "type": "call", "data": { "from": "user", "to": "preview", "label": "select playlist" } },
    { "type": "call", "data": { "from": "preview", "to": "video", "label": "check canPlayType()" } },
    { "type": "fragment", "data": { "fragment": { "kind": "alt", "branches": [
      { "label": "Native HLS support", "messages": [
        { "type": "call", "data": { "from": "preview", "to": "video", "label": "src = playlist.uri" } },
        { "type": "call", "data": { "from": "video", "to": "playlist", "label": "load manifest" } }
      ] },
      { "label": "HLS.js fallback", "messages": [
        { "type": "call", "data": { "from": "preview", "to": "hlsjs", "label": "new Hls()" } },
        { "type": "call", "data": { "from": "hlsjs", "to": "video", "label": "attachMedia()" } },
        { "type": "call", "data": { "from": "hlsjs", "to": "playlist", "label": "loadSource()" } }
      ] }
    ] } } }
  ]
}"#;
    let diagram = parse_json(data_only).expect("auto-heal missing type");
    match diagram {
        Diagram::Sequence(seq) => {
            assert_eq!(seq.actors.len(), 5);
            assert_eq!(seq.messages.len(), 3);
        }
        other => panic!("Expected Sequence diagram, got {:?}", other),
    }

    let wrapped = format!(r#"{{"data": {data_only}}}"#);
    let diagram = parse_json(&wrapped).expect("auto-heal missing type with data wrapper");
    match diagram {
        Diagram::Sequence(seq) => {
            assert_eq!(seq.actors.len(), 5);
            assert_eq!(seq.messages.len(), 3);
        }
        other => panic!("Expected Sequence diagram, got {:?}", other),
    }
}

#[test]
fn parse_json_auto_heals_fragment_shorthand_from_log() {
    let diagram = r#"{
  "type": "sequence",
  "data": {
    "actors": [
      { "id": "user", "label": "User", "kind": "user" },
      { "id": "sniffer_ui", "label": "Sniffer UI", "kind": "service" },
      { "id": "preview_component", "label": "PlaylistPreview", "kind": "service" },
      { "id": "hls_js", "label": "hls.js", "kind": "service" },
      { "id": "video_element", "label": "Video Element", "kind": "generic" },
      { "id": "external_playlist", "label": "External Playlist URI", "kind": "external" }
    ],
    "messages": [
      { "type": "call", "data": { "from": "user", "to": "sniffer_ui", "label": "Select playlist" } },
      { "type": "call", "data": { "from": "sniffer_ui", "to": "preview_component", "label": "Render preview" } },
      { "type": "call", "data": { "from": "preview_component", "to": "hls_js", "label": "new Hls()" } },
      { "type": "call", "data": { "from": "hls_js", "to": "video_element", "label": "attachMedia()" } },
      { "type": "call", "data": { "from": "hls_js", "to": "external_playlist", "label": "loadSource(uri)" } },
      { "type": "alt", "data": { "branches": [
        { "label": "Native HLS support", "messages": [
          { "type": "call", "data": { "from": "preview_component", "to": "video_element", "label": "video.src = uri" } }
        ] }
      ] } },
      { "type": "return", "data": { "from": "hls_js", "to": "preview_component", "label": "MANIFEST_PARSED" } },
      { "type": "call", "data": { "from": "preview_component", "to": "sniffer_ui", "label": "Show preview" } },
      { "type": "alt", "data": { "branches": [
        { "label": "Error", "messages": [
          { "type": "call", "data": { "from": "hls_js", "to": "preview_component", "label": "ERROR event" } },
          { "type": "call", "data": { "from": "preview_component", "to": "sniffer_ui", "label": "Show error state" } }
        ] }
      ] } }
    ]
  }
}"#;
    let diagram = parse_json(diagram).expect("auto-heal fragment shorthand");
    match diagram {
        Diagram::Sequence(seq) => {
            assert_eq!(seq.actors.len(), 6);
            let fragment_count = seq
                .messages
                .iter()
                .filter(|msg| matches!(msg, Message::Fragment { .. }))
                .count();
            assert_eq!(fragment_count, 2);
        }
        other => panic!("Expected Sequence diagram, got {:?}", other),
    }
}

#[test]
fn sequence_rendering() {
    let seq = SequenceDiagram {
        actors: vec![
            Actor {
                id: "User".into(),
                label: "User".into(),
                kind: ActorKind::User,
                style: NodeStyle::default(),
            },
            Actor {
                id: "API".into(),
                label: "API".into(),
                kind: ActorKind::Service,
                style: NodeStyle::default(),
            },
        ],
        messages: vec![
            Message::Call {
                from: "User".into(),
                to: "API".into(),
                label: "POST /login".into(),
                is_async: true,
                style: EdgeStyle::default(),
            },
            Message::Return {
                from: "API".into(),
                to: "User".into(),
                label: Some("200".into()),
            },
        ],
        metadata: Metadata::default(),
    };

    let d2 = D2Renderer.render(&Diagram::Sequence(seq.clone())).unwrap();
    let mermaid = MermaidRenderer.render(&Diagram::Sequence(seq)).unwrap();
    assert!(d2.contains("POST /login"));
    assert!(mermaid.contains("sequenceDiagram"));
}

#[test]
fn entity_rendering() {
    let entity = EntityDiagram {
        entities: vec![Entity {
            id: "users".into(),
            label: "Users".into(),
            attributes: vec![
                Attribute {
                    name: "id".into(),
                    type_name: "uuid".into(),
                    is_key: true,
                    nullable: false,
                },
                Attribute {
                    name: "email".into(),
                    type_name: "text".into(),
                    is_key: false,
                    nullable: false,
                },
            ],
        }],
        relationships: vec![],
    };

    let d2 = D2Renderer.render(&Diagram::Entity(entity.clone())).unwrap();
    let mermaid = MermaidRenderer.render(&Diagram::Entity(entity)).unwrap();
    assert!(d2.contains("sql_table"));
    assert!(mermaid.contains("erDiagram"));
}

#[test]
fn flow_json_complex_example() {
    let flow = parse_flow_json(json!({
        "type": "flow",
        "data": {
            "direction": "LR",
            "nodes": [
                { "id": "bucket", "label": "IndexedDBBucket", "kind": "service" },
                { "id": "meta", "label": "BucketMeta", "kind": "database" },
                { "id": "measure", "label": "measureBucketUsage", "kind": "service" },
                { "id": "estimate", "label": "getStorageEstimate", "kind": "service" },
                { "id": "browser", "label": "Browser API", "kind": "generic" }
            ],
            "edges": [
                { "from": "bucket", "to": "meta", "label": "trackUsage()", "style": { "color": "blue" } },
                { "from": "bucket", "to": "measure", "label": "fallback measurement", "style": { "color": "orange" }, "dashed": true },
                { "from": "measure", "to": "meta", "label": "update metadata" },
                { "from": "estimate", "to": "browser", "label": "navigator.storage.estimate" },
                { "from": "browser", "to": "estimate", "label": "quota/usage data" }
            ],
            "groups": [
                { "id": "tracking", "label": "Storage Tracking", "members": ["bucket", "meta", "measure"] },
                { "id": "estimation", "label": "Quota Estimation", "members": ["estimate", "browser"] }
            ]
        }
    }));
    let mermaid = MermaidRenderer.render(&Diagram::Flow(flow)).unwrap();
    assert_contains_all(
        &mermaid,
        &["flowchart LR", "IndexedDBBucket", "Storage Tracking"],
    );
    assert!(mermaid.contains("subgraph tracking[\"Storage Tracking\"]"));
}

#[test]
fn flow_json_unknown_kind_falls_back() {
    let flow = parse_flow_json(json!({
        "type": "flow",
        "data": {
            "direction": "LR",
            "nodes": [
                { "id": "mystery", "label": "Mystery", "kind": "widget" }
            ]
        }
    }));
    assert_eq!(flow.nodes[0].kind, NodeKind::Generic);
}

#[test]
fn sequence_json_basic() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "api", "label": "API", "kind": "service" },
                { "id": "db", "label": "DB", "kind": "database" }
            ],
            "messages": [
                { "type": "call", "data": { "from": "user", "to": "api", "label": "POST /login" } },
                { "type": "call", "data": { "from": "api", "to": "db", "label": "SELECT user" } },
                { "type": "return", "data": { "from": "db", "to": "api", "label": "user record" } },
                { "type": "call", "data": { "from": "api", "to": "user", "label": "200 OK" } }
            ]
        }
    }));
    assert_eq!(seq.actors.len(), 3);
    assert_eq!(seq.messages.len(), 4);
}

#[test]
fn sequence_json_file_and_unknown_actor_kinds() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "file", "label": "File", "kind": "file" },
                { "id": "svc", "label": "Service", "kind": "service" },
                { "id": "mystery", "label": "Mystery", "kind": "widget" }
            ],
            "messages": []
        }
    }));
    assert_eq!(seq.actors[0].kind, ActorKind::File);
    assert_eq!(seq.actors[2].kind, ActorKind::Generic);
}

#[test]
fn sequence_json_with_notes() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "api", "label": "API", "kind": "service" }
            ],
            "messages": [
                { "type": "call", "data": { "from": "user", "to": "api", "label": "Request" } },
                { "type": "note", "data": { "target": ["user", "api"], "text": "This is a note" } },
                { "type": "call", "data": { "from": "api", "to": "user", "label": "Response" } }
            ]
        }
    }));
    assert_eq!(seq.messages.len(), 3);
}

#[test]
fn sequence_json_alt_else_labels_render() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "api", "label": "API", "kind": "service" }
            ],
            "messages": [
                {
                    "type": "fragment",
                    "data": {
                        "fragment": {
                            "kind": "alt",
                            "branches": [
                                { "label": "success", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "Ping" } }
                                ]},
                                { "label": "failure", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "Retry" } }
                                ]}
                            ]
                        }
                    }
                }
            ]
        }
    }));
    let fragment = match &seq.messages[0] {
        Message::Fragment { fragment } => fragment,
        other => panic!("Expected Fragment message, got {:?}", other),
    };
    assert_eq!(fragment.kind, FragmentKind::Alt);
    assert_eq!(fragment.branches.len(), 2);
    assert_eq!(fragment.branches[0].label.as_deref(), Some("success"));
    assert_eq!(fragment.branches[1].label.as_deref(), Some("failure"));

    let (d2, mermaid) = render_sequence_outputs(&seq);
    assert_contains_all(&d2, &["alt \"success\"", "else \"failure\"", "end"]);
    assert_contains_all(
        &mermaid,
        &["sequenceDiagram", "alt success", "else failure", "end"],
    );
}

#[test]
fn sequence_json_alt_multiple_else_branches() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "api", "label": "API", "kind": "service" }
            ],
            "messages": [
                {
                    "type": "fragment",
                    "data": {
                        "fragment": {
                            "kind": "alt",
                            "branches": [
                                { "label": "primary", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "A" } }
                                ]},
                                { "label": "secondary", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "B" } }
                                ]},
                                { "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "C" } }
                                ]}
                            ]
                        }
                    }
                }
            ]
        }
    }));
    let (d2, mermaid) = render_sequence_outputs(&seq);

    assert_eq!(count_lines_starting_with(&d2, "else"), 2);
    assert_eq!(count_lines_starting_with(&mermaid, "else"), 2);
    assert!(!mermaid.contains("else Alternative"));
}

#[test]
fn sequence_json_par_branches_render_and() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "api", "label": "API", "kind": "service" }
            ],
            "messages": [
                {
                    "type": "fragment",
                    "data": {
                        "fragment": {
                            "kind": "par",
                            "branches": [
                                { "label": "fast", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "A" } }
                                ]},
                                { "label": "slow", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "B" } }
                                ]},
                                { "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "C" } }
                                ]}
                            ]
                        }
                    }
                }
            ]
        }
    }));
    let (d2, mermaid) = render_sequence_outputs(&seq);

    assert_contains_all(&d2, &["par \"fast\"", "and \"slow\"", "end"]);
    assert_contains_all(&mermaid, &["par fast", "and slow", "end"]);
    assert_eq!(count_lines_starting_with(&d2, "and"), 2);
    assert_eq!(count_lines_starting_with(&mermaid, "and"), 2);
}

#[test]
fn sequence_json_critical_option_branches() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "api", "label": "API", "kind": "service" }
            ],
            "messages": [
                {
                    "type": "fragment",
                    "data": {
                        "fragment": {
                            "kind": "critical",
                            "branches": [
                                { "label": "transaction", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "begin" } }
                                ]},
                                { "label": "rollback", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "abort" } }
                                ]}
                            ]
                        }
                    }
                }
            ]
        }
    }));
    let (d2, mermaid) = render_sequence_outputs(&seq);

    assert_contains_all(
        &d2,
        &["critical \"transaction\"", "option \"rollback\"", "end"],
    );
    assert_contains_all(
        &mermaid,
        &["critical transaction", "option rollback", "end"],
    );
    assert_eq!(count_lines_starting_with(&d2, "option"), 1);
    assert_eq!(count_lines_starting_with(&mermaid, "option"), 1);
}

#[test]
fn sequence_json_opt_loop_break_render() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "api", "label": "API", "kind": "service" }
            ],
            "messages": [
                {
                    "type": "fragment",
                    "data": {
                        "fragment": {
                            "kind": "opt",
                            "branches": [
                                { "label": "cached", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "hit" } }
                                ]},
                                { "label": "retry", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "retry" } }
                                ]},
                                { "label": "fatal", "messages": [
                                    { "type": "call", "data": { "from": "user", "to": "api", "label": "abort" } }
                                ]}
                            ]
                        }
                    }
                }
            ]
        }
    }));
    let (d2, mermaid) = render_sequence_outputs(&seq);

    assert_contains_all(&d2, &["opt \"cached\"", "else \"retry\"", "else \"fatal\""]);
    assert_contains_all(&mermaid, &["opt cached", "else retry", "else fatal"]);
    assert_eq!(count_lines_starting_with(&d2, "end"), 1);
    assert_eq!(count_lines_starting_with(&mermaid, "end"), 1);
}

#[test]
fn sequence_json_complex() {
    let seq = parse_sequence_json(json!({
        "type": "sequence",
        "data": {
            "actors": [
                { "id": "user", "label": "User", "kind": "user" },
                { "id": "download", "label": "Download Epic", "kind": "service" },
                { "id": "bucket", "label": "IndexedDBBucket", "kind": "service" },
                { "id": "tracker", "label": "trackBucketUsage", "kind": "service" },
                { "id": "meta", "label": "Bucket Metadata", "kind": "database" },
                { "id": "storage", "label": "Storage Stats", "kind": "service" }
            ],
            "messages": [
                { "type": "call", "data": { "from": "user", "to": "download", "label": "Start download" } },
                { "type": "call", "data": { "from": "download", "to": "bucket", "label": "write(chunk)" } },
                { "type": "call", "data": { "from": "bucket", "to": "tracker", "label": "trackBucketUsage(id, bytes)" } },
                { "type": "call", "data": { "from": "tracker", "to": "meta", "label": "update metadata" } },
                { "type": "note", "data": { "target": ["bucket", "meta"], "text": "Atomic update required" } },
                { "type": "call", "data": { "from": "storage", "to": "meta", "label": "query usage stats" } },
                { "type": "call", "data": { "from": "meta", "to": "storage", "label": "return stats" } }
            ]
        }
    }));
    assert_eq!(seq.actors.len(), 6);
    assert_eq!(seq.messages.len(), 7);
}
