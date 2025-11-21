//! Git tag tool

use kodegen_mcp_tool::{Tool, ToolExecutionContext, error::McpError};
use kodegen_mcp_schema::git::{GitTagArgs, GitTagPromptArgs};
use rmcp::model::{PromptArgument, PromptMessage, Content};
use serde_json::json;
use std::path::Path;

/// Tool for managing repository tags
#[derive(Clone)]
pub struct GitTagTool;

impl Tool for GitTagTool {
    type Args = GitTagArgs;
    type PromptArgs = GitTagPromptArgs;

    fn name() -> &'static str {
        kodegen_mcp_schema::git::GIT_TAG
    }

    fn description() -> &'static str {
        "Manage repository tags. Operations: 'create' to create tag, \
         'delete' to remove tag, 'list' to show all tags."
    }

    fn read_only() -> bool {
        false // Can modify refs
    }

    fn destructive() -> bool {
        true // Delete operation removes refs
    }

    fn idempotent() -> bool {
        false // Cannot create duplicate tags (unless forced)
    }

    async fn execute(&self, args: Self::Args, _ctx: ToolExecutionContext) -> Result<Vec<Content>, McpError> {
        let path = Path::new(&args.path);

        // Open repository
        let repo = crate::open_repo(path)
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

        let mut contents = Vec::new();

        if args.operation.as_str() == "create" {
            let name = args.name.ok_or_else(|| {
                McpError::Other(anyhow::anyhow!("Tag name required for create operation"))
            })?;

            let opts = crate::TagOpts {
                name: name.clone(),
                message: args.message.clone(),
                target: args.target.clone(),
                force: args.force,
            };

            let repo_clone = repo.clone();
            let tag_info = tokio::task::spawn_blocking(move || {
                let repo_inner = repo_clone.clone_inner();
                let tag_ref_name = format!("refs/tags/{}", opts.name);

                // Resolve target commit
                let target = if let Some(ref target_str) = opts.target {
                    repo_inner
                        .rev_parse_single(target_str.as_bytes().as_bstr())
                        .map_err(|e| anyhow::anyhow!("Invalid target '{target_str}': {e}"))?
                        .into()
                } else {
                    // Default to HEAD
                    let mut head = repo_inner.head().map_err(|e| anyhow::anyhow!("{e}"))?;
                    head.try_peel_to_id()
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                        .ok_or_else(|| anyhow::anyhow!("HEAD does not point to a commit"))?
                        .detach()
                };

                // Check if tag exists
                if !opts.force
                    && repo_inner
                        .refs
                        .find(tag_ref_name.as_bytes().as_bstr())
                        .is_ok()
                {
                    return Err(anyhow::anyhow!("Tag '{}' already exists", opts.name));
                }

                // Create tag reference
                let is_annotated = opts.message.is_some();

                // For annotated tags, use tag method
                use gix::bstr::ByteSlice;
                if is_annotated {
                    let message = opts.message.as_deref().unwrap_or("");
                    let config = repo_inner.config_snapshot();
                    let name_val = config
                        .string("user.name")
                        .ok_or_else(|| anyhow::anyhow!("Git user.name not configured"))?;
                    let email_val = config
                        .string("user.email")
                        .ok_or_else(|| anyhow::anyhow!("Git user.email not configured"))?;

                    let signature = gix::actor::Signature {
                        name: name_val.into_owned(),
                        email: email_val.into_owned(),
                        time: gix::date::Time::now_local_or_utc(),
                    };

                    let time_str = signature.time.to_string();
                    let sig_ref = gix::actor::SignatureRef {
                        name: signature.name.as_bstr(),
                        email: signature.email.as_bstr(),
                        time: &time_str,
                    };

                    repo_inner
                        .tag(
                            &opts.name,
                            target,
                            gix::objs::Kind::Commit,
                            Some(sig_ref),
                            message,
                            if opts.force {
                                gix::refs::transaction::PreviousValue::Any
                            } else {
                                gix::refs::transaction::PreviousValue::MustNotExist
                            },
                        )
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                } else {
                    let ref_name = gix::refs::FullName::try_from(tag_ref_name.as_bytes().as_bstr())
                        .map_err(|e| anyhow::anyhow!("{e}"))?;

                    let edit = gix::refs::transaction::RefEdit {
                        change: gix::refs::transaction::Change::Update {
                            log: gix::refs::transaction::LogChange::default(),
                            expected: if opts.force {
                                gix::refs::transaction::PreviousValue::Any
                            } else {
                                gix::refs::transaction::PreviousValue::MustNotExist
                            },
                            new: gix::refs::Target::Object(target),
                        },
                        name: ref_name,
                        deref: false,
                    };

                    repo_inner
                        .refs
                        .transaction()
                        .prepare(
                            vec![edit],
                            gix::lock::acquire::Fail::Immediately,
                            gix::lock::acquire::Fail::Immediately,
                        )
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                        .commit(None)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                }

                // Get tag info
                let commit = repo_inner
                    .find_object(target)
                    .map_err(|e| anyhow::anyhow!("{e}"))?
                    .try_into_commit()
                    .map_err(|_| anyhow::anyhow!("Target is not a commit"))?;

                let commit_time = commit.time().map_err(|e| anyhow::anyhow!("{e}"))?;
                let timestamp = chrono::DateTime::from_timestamp(commit_time.seconds, 0)
                    .unwrap_or_else(chrono::Utc::now);

                Ok(crate::TagInfo {
                    name: opts.name,
                    message: opts.message,
                    target_commit: target.to_string(),
                    timestamp,
                    is_annotated,
                })
            })
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

            // Determine tag type
            let tag_type = if tag_info.is_annotated {
                "annotated"
            } else {
                "lightweight"
            };

            // Terminal summary
            let summary = format!(
                "\x1b[35m\u{1F3F7} Tag Created\x1b[0m\n\
                 Name: {} ({})\n\
                 Target: {}",
                name, tag_type,
                &tag_info.target_commit[..7.min(tag_info.target_commit.len())]
            );
            contents.push(Content::text(summary));

            // JSON metadata
            let metadata = json!({
                "success": true,
                "operation": "create",
                "name": name,
                "is_annotated": tag_info.is_annotated,
                "target_commit": tag_info.target_commit,
                "message": tag_info.message
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));
        } else if args.operation.as_str() == "delete" {
            let name = args.name.ok_or_else(|| {
                McpError::Other(anyhow::anyhow!("Tag name required for delete operation"))
            })?;

            let repo_clone = repo.clone();
            let name_clone = name.clone();
            tokio::task::spawn_blocking(move || -> Result<(), anyhow::Error> {
                use gix::bstr::ByteSlice;
                let repo_inner = repo_clone.clone_inner();
                let tag_ref_name = format!("refs/tags/{}", name_clone);

                // Check if tag exists
                repo_inner
                    .refs
                    .find(tag_ref_name.as_bytes().as_bstr())
                    .map_err(|_| anyhow::anyhow!("Tag '{}' not found", name_clone))?;

                // Delete the tag using transaction
                let ref_name = gix::refs::FullName::try_from(tag_ref_name.as_bytes().as_bstr())
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                let edit = gix::refs::transaction::RefEdit {
                    change: gix::refs::transaction::Change::Delete {
                        expected: gix::refs::transaction::PreviousValue::Any,
                        log: gix::refs::transaction::RefLog::AndReference,
                    },
                    name: ref_name,
                    deref: false,
                };

                repo_inner
                    .refs
                    .transaction()
                    .prepare(
                        vec![edit],
                        gix::lock::acquire::Fail::Immediately,
                        gix::lock::acquire::Fail::Immediately,
                    )
                    .map_err(|e| anyhow::anyhow!("{e}"))?
                    .commit(None)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                Ok(())
            })
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

            // Terminal summary
            let summary = format!(
                "\x1b[32m\u{2713} Tag Deleted\x1b[0m\n\
                 {} removed from repository",
                name
            );
            contents.push(Content::text(summary));

            // JSON metadata
            let metadata = json!({
                "success": true,
                "operation": "delete",
                "name": name
            });
            let json_str = serde_json::to_string_pretty(&metadata)
                .unwrap_or_else(|_| "{}".to_string());
            contents.push(Content::text(json_str));
        } else if args.operation.as_str() == "list" {
            let repo_clone = repo.clone();
            let tags = tokio::task::spawn_blocking(move || {
                let repo_inner = repo_clone.clone_inner();
                let mut tags = Vec::new();

                // Iterate over all tag references
                let refs_platform = repo_inner
                    .references()
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                let tag_refs = refs_platform
                    .prefixed("refs/tags/")
                    .map_err(|e| anyhow::anyhow!("{e}"))?;

                for reference in tag_refs {
                    let mut reference = reference.map_err(|e| anyhow::anyhow!("{e}"))?;

                    let name_bstr = reference.name().as_bstr();
                    if !name_bstr.starts_with(b"refs/tags/") {
                        continue;
                    }

                    let tag_name = name_bstr
                        .strip_prefix(b"refs/tags/")
                        .and_then(|n| std::str::from_utf8(n).ok())
                        .ok_or_else(|| anyhow::anyhow!("Invalid tag name"))?
                        .to_string();

                    // Get target
                    let target_id = reference
                        .peel_to_id()
                        .map_err(|e| anyhow::anyhow!("{e}"))?;

                    // Try to get tag object for annotated tags
                    let (message, is_annotated, timestamp) = if let Ok(obj) =
                        repo_inner.find_object(target_id)
                    {
                        if let Ok(tag_obj) = obj.try_into_tag() {
                            let tag_ref = tag_obj.decode().ok();
                            let msg = tag_ref.as_ref().map(|t| t.message.to_string());
                            let ts = if let Some(ref tag) = tag_ref {
                                if let Some(tagger) = &tag.tagger {
                                    if let Ok(time) = tagger.time() {
                                        chrono::DateTime::from_timestamp(time.seconds, 0)
                                            .unwrap_or_else(chrono::Utc::now)
                                    } else {
                                        chrono::Utc::now()
                                    }
                                } else {
                                    chrono::Utc::now()
                                }
                            } else {
                                chrono::Utc::now()
                            };
                            (msg, true, ts)
                        } else if let Ok(obj2) = repo_inner.find_object(target_id) {
                            if let Ok(commit) = obj2.try_into_commit() {
                                let ts = commit
                                    .time()
                                    .ok()
                                    .unwrap_or_else(gix::date::Time::now_local_or_utc);
                                let ts = chrono::DateTime::from_timestamp(ts.seconds, 0)
                                    .unwrap_or_else(chrono::Utc::now);
                                (None, false, ts)
                            } else {
                                (None, false, chrono::Utc::now())
                            }
                        } else {
                            (None, false, chrono::Utc::now())
                        }
                    } else {
                        (None, false, chrono::Utc::now())
                    };

                    tags.push(crate::TagInfo {
                        name: tag_name,
                        message,
                        target_commit: target_id.to_string(),
                        timestamp,
                        is_annotated,
                    });
                }

                Ok::<Vec<crate::TagInfo>, anyhow::Error>(tags)
            })
            .await
            .map_err(|e| McpError::Other(anyhow::anyhow!("Task execution failed: {e}")))?
            .map_err(|e| McpError::Other(anyhow::anyhow!("{e}")))?;

            // Terminal summary
            let mut summary = format!(
                "\x1b[34m\u{1F4CC} Tags ({})\x1b[0m",
                tags.len()
            );

            if tags.is_empty() {
                summary.push_str("\n  No tags in repository");

                contents.push(Content::text(summary));

                let metadata = json!({
                    "success": true,
                    "operation": "list",
                    "count": 0,
                    "tags": []
                });
                let json_str = serde_json::to_string_pretty(&metadata)
                    .unwrap_or_else(|_| "{}".to_string());
                contents.push(Content::text(json_str));
            } else {
                // Sort tags for consistent output
                let mut sorted_tags = tags;
                sorted_tags.sort_by(|a, b| b.timestamp.cmp(&a.timestamp)); // Newest first

                for tag in sorted_tags.iter().take(20) {
                    let tag_type = if tag.is_annotated {
                        "annotated"
                    } else {
                        "lightweight"
                    };
                    let short_commit = &tag.target_commit[..7.min(tag.target_commit.len())];
                    summary.push_str(&format!(
                        "\n  {} \u{27A1} {} ({})",
                        tag.name, short_commit, tag_type
                    ));
                }

                if sorted_tags.len() > 20 {
                    summary.push_str(&format!("\n  ... and {} more", sorted_tags.len() - 20));
                }

                contents.push(Content::text(summary));

                // JSON metadata - include all tags, not just the displayed 20
                let tag_list: Vec<serde_json::Value> = sorted_tags
                    .iter()
                    .map(|t| {
                        json!({
                            "name": t.name,
                            "is_annotated": t.is_annotated,
                            "target_commit": t.target_commit,
                            "message": t.message,
                            "timestamp": t.timestamp.to_rfc3339()
                        })
                    })
                    .collect();

                let metadata = json!({
                    "success": true,
                    "operation": "list",
                    "count": sorted_tags.len(),
                    "tags": tag_list
                });
                let json_str = serde_json::to_string_pretty(&metadata)
                    .unwrap_or_else(|_| "{}".to_string());
                contents.push(Content::text(json_str));
            }
        } else {
            return Err(McpError::Other(anyhow::anyhow!(
                "Invalid tag operation: {}. Use 'create', 'delete', or 'list'",
                args.operation
            )));
        }

        Ok(contents)
    }

    fn prompt_arguments() -> Vec<PromptArgument> {
        vec![]
    }

    async fn prompt(&self, _args: Self::PromptArgs) -> Result<Vec<PromptMessage>, McpError> {
        Ok(vec![])
    }
}
