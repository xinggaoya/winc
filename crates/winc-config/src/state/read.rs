//! 状态文件的读取：把 `state.json` 的内容读成 [`State`]。
//!
//! 各项读出来后先做范围与类型的检查，不合格的丢掉，其余照常读入。

use super::*;

impl State {
    /// 解析状态文件内容；格式错误或顶层不是对象时返回 `None`。
    pub(super) fn try_parse(text: &str) -> Option<State> {
        let root = Json::parse(text)?;
        let root = root.as_object()?;

        let paths = |value: Option<&Json>| -> Vec<PathBuf> {
            value
                .and_then(Json::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Json::as_str)
                        .filter(|s| !s.is_empty())
                        .map(PathBuf::from)
                        .collect()
                })
                .unwrap_or_default()
        };

        let mut recent = Vec::new();
        for path in paths(root.get("recent")) {
            if !recent.contains(&path) && recent.len() < MAX_RECENT {
                recent.push(path);
            }
        }
        let mut recent_files = Vec::new();
        for path in paths(root.get("recentFiles")) {
            if !recent_files.contains(&path) && recent_files.len() < MAX_RECENT_FILES {
                recent_files.push(path);
            }
        }

        let earlier = std::cell::RefCell::new(Vec::new());
        // 已保存的文件写成路径字符串，未保存过的新文件写成 `{"untitled": 备份文件名}`。
        let tab = |value: &Json| -> Option<Tab> {
            if let Some(path) = value.as_str() {
                return (!path.is_empty()).then(|| Tab::File(PathBuf::from(path)));
            }
            let name = value.as_object()?.get("untitled")?.as_str()?;
            (!name.is_empty()).then(|| Tab::Untitled(name.to_owned()))
        };
        let session = |value: &Json| -> Option<Session> {
            let s = value.as_object()?;
            let folder = PathBuf::from(s.get("folder")?.as_str().filter(|f| !f.is_empty())?);
            let tabs: Vec<Tab> = s
                .get("files")
                .and_then(Json::as_array)
                .unwrap_or_default()
                .iter()
                .filter_map(tab)
                .collect();
            let active = s
                .get("active")
                .and_then(Json::as_number)
                .filter(|n| *n >= 0.0 && n.fract() == 0.0)
                .map(|n| n as usize)
                .filter(|&i| i < tabs.len());
            let bookmarks = s
                .get("bookmarks")
                .and_then(Json::as_object)
                .map(|files| {
                    files
                        .iter()
                        .filter(|(path, _)| !path.is_empty())
                        .map(|(path, lines)| {
                            let lines = lines
                                .as_array()
                                .unwrap_or_default()
                                .iter()
                                .filter_map(Json::as_number)
                                .filter(|n| *n >= 0.0 && n.fract() == 0.0)
                                .map(|n| n as usize)
                                .collect();
                            (PathBuf::from(path), lines)
                        })
                        .filter(|(_, lines): &(PathBuf, Vec<usize>)| !lines.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let folds = s
                .get("folds")
                .and_then(Json::as_object)
                .map(|files| {
                    let line = |n: &Json| {
                        let n = n.as_number().filter(|n| *n >= 0.0 && n.fract() == 0.0)?;
                        Some(n as usize)
                    };
                    let block = |pair: &Json| match pair.as_array()? {
                        [start, end] => Some((line(start)?, line(end)?)),
                        _ => None,
                    };
                    files
                        .iter()
                        .filter(|(path, _)| !path.is_empty())
                        .map(|(path, blocks)| {
                            let blocks = blocks.as_array().unwrap_or_default();
                            let blocks = blocks.iter().filter_map(block);
                            (PathBuf::from(path), blocks.filter(|(s, e)| s < e).collect())
                        })
                        .filter(|(_, blocks): &(PathBuf, Vec<(usize, usize)>)| !blocks.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            let outline = s
                .get("outline")
                .and_then(Json::as_object)
                .map(|files| {
                    files
                        .iter()
                        .filter(|(path, _)| !path.is_empty())
                        .map(|(path, symbols)| {
                            let symbols = symbols
                                .as_array()
                                .unwrap_or_default()
                                .iter()
                                .filter_map(Json::as_str)
                                .filter(|s| !s.is_empty())
                                .map(str::to_owned)
                                .collect();
                            (PathBuf::from(path), symbols)
                        })
                        .filter(|(_, symbols): &(PathBuf, Vec<String>)| !symbols.is_empty())
                        .collect()
                })
                .unwrap_or_default();
            // 早先各目录的现场里记的阅读位置：并入 `places`。
            for key in ["pdfPlaces", "bookPlaces"] {
                earlier.borrow_mut().extend(read_views(s.get(key)));
            }
            let terminal = s.get("terminal").and_then(Json::as_object).and_then(|t| {
                let number = |key| {
                    t.get(key)
                        .and_then(Json::as_number)
                        .filter(|n| n.is_finite() && *n >= 0.0)
                };
                let tabs: Vec<PathBuf> = t
                    .get("tabs")?
                    .as_array()?
                    .iter()
                    .filter_map(Json::as_str)
                    .map(PathBuf::from)
                    .collect();
                let shells = t.get("shells").and_then(Json::as_array).map(|shells| {
                    let shells = shells.iter().map(|s| s.as_str().unwrap_or_default());
                    shells.map(str::to_owned).collect()
                });
                let names = t.get("names").and_then(Json::as_array).map(|names| {
                    let names = names.iter().map(|n| n.as_str().unwrap_or_default());
                    names.map(str::to_owned).collect()
                });
                let groups = read_terminal_groups(t, tabs.len());
                (!tabs.is_empty()).then(|| TerminalSession {
                    active: number("active").map_or(0, |n| (n as usize).min(tabs.len() - 1)),
                    tabs,
                    names: names.unwrap_or_default(),
                    shells: shells.unwrap_or_default(),
                    open: t.get("open").and_then(Json::as_bool).unwrap_or(false),
                    height: number("height").unwrap_or(0.0) as f32,
                    width: number("width").unwrap_or(0.0) as f32,
                    maximized: t.get("maximized").and_then(Json::as_bool).unwrap_or(false),
                    groups,
                })
            });
            let folders = (s
                .get("folders")
                .and_then(Json::as_array)
                .into_iter()
                .flatten())
            .filter_map(Json::as_str)
            .map(PathBuf::from)
            .collect();
            let readonly = (s
                .get("readOnly")
                .and_then(Json::as_array)
                .into_iter()
                .flatten())
            .filter_map(Json::as_str)
            .map(PathBuf::from)
            .collect();
            Some(Session {
                folder,
                tabs,
                active,
                bookmarks,
                outline,
                folds,
                terminal,
                run: s.get("run").and_then(Json::as_str).map(str::to_owned),
                folders,
                readonly,
                readonly_all: s.get("readOnlyAll").and_then(Json::as_bool) == Some(true),
                name: s.get("name").and_then(Json::as_str).map(str::to_owned),
                project: s.get("project").and_then(Json::as_str).map(PathBuf::from),
                sidebar_hidden: (s.get("sidebarHidden").and_then(Json::as_bool)).unwrap_or(false),
                window: (s.get("window").and_then(Json::as_object)).and_then(window_frame),
                panes: s.get("panes").and_then(read_panes),
            })
        };

        let badges = root
            .get("badges")
            .and_then(Json::as_object)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|(path, color)| {
                        let color = color.as_number().filter(|n| n.fract() == 0.0)?;
                        Some((PathBuf::from(path), color as usize))
                    })
                    .collect()
            })
            .unwrap_or_default();

        let tool_choices = root
            .get("toolChoices")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(|item| {
                let item = item.as_object()?;
                let folder = item.get("folder")?.as_str().filter(|f| !f.is_empty())?;
                Some((PathBuf::from(folder), item.get("run")?.as_bool()?))
            })
            .take(MAX_TOOL_CHOICES)
            .collect();

        let approved_commands = root
            .get("approvedCommands")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(|item| {
                let item = item.as_object()?;
                let folder = item.get("folder")?.as_str().filter(|f| !f.is_empty())?;
                let command = item.get("command")?.as_str()?;
                Some((PathBuf::from(folder), command.to_owned()))
            })
            .take(MAX_TOOL_CHOICES)
            .collect();

        // 指纹是 64 位，存成十六进制字符串：JSON 的数字按 f64 读，精度不够，装不下。
        let trusted_repos = root
            .get("trustedRepos")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(|item| {
                let item = item.as_object()?;
                let folder = item.get("folder")?.as_str().filter(|f| !f.is_empty())?;
                let fingerprint =
                    u64::from_str_radix(item.get("fingerprint")?.as_str()?, 16).ok()?;
                Some((PathBuf::from(folder), fingerprint))
            })
            .take(MAX_TOOL_CHOICES)
            .collect();

        let remote_keys = root
            .get("remoteKeys")
            .and_then(Json::as_object)
            .map(|items| {
                (items.iter())
                    .filter_map(|(target, key)| Some((target.clone(), key.as_str()?.to_owned())))
                    .filter(|(_, key)| !key.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let vim_marks = vim_marks(root);
        let vim_commands = vim_history(root, "vimCommands");
        let vim_searches = vim_history(root, "vimSearches");

        let mut state = State {
            recent,
            recent_files,
            badges,
            tool_choices,
            approved_commands,
            trusted_repos,
            remote_keys,
            vim_marks,
            vim_commands,
            vim_searches,
            ..State::default()
        };
        // 旧文件和手工修改过的文件里，颜色可能缺失、重复或越界；读入后统一理顺。
        state.assign_badges();
        for s in root
            .get("sessions")
            .and_then(Json::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(session)
        {
            if !state.sessions.iter().any(|old| old.folder == s.folder)
                && state.sessions.len() < MAX_SESSIONS
            {
                state.sessions.push(s);
            }
        }
        state.last = root
            .get("last")
            .and_then(Json::as_str)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);
        // 旧版曾默认排成树，只记录选了平铺（`commitFlat`）；默认平铺后该项不再读入。
        state.commit_tree = root
            .get("commitTree")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        state.commit_sort = root
            .get("commitSort")
            .and_then(Json::as_str)
            .unwrap_or_default()
            .to_owned();
        state.commit_all_repositories = root.get("commitAllRepositories").and_then(Json::as_bool);
        state.log_branches = root.get("logBranches").and_then(Json::as_bool);
        state.log_files_tree = root.get("logFilesTree").and_then(Json::as_bool);
        state.diff_files_tree = root.get("diffFilesTree").and_then(Json::as_bool);
        state.search_tree = (root.get("searchTree").and_then(Json::as_bool)).unwrap_or(false);
        state.log_columns = (root.get("logColumns").and_then(Json::as_array)).map(|columns| {
            let names = columns.iter().filter_map(Json::as_str);
            names.map(str::to_owned).collect()
        });
        state.commit_messages = (root.get("commitMessages").and_then(Json::as_array))
            .unwrap_or_default()
            .iter()
            .filter_map(Json::as_str)
            .filter(|m| !m.is_empty())
            .take(MAX_COMMIT_MESSAGES)
            .map(str::to_owned)
            .collect();
        state.commit_check_todo = root.get("commitCheckTodo").and_then(Json::as_bool) == Some(true);
        state.commit_reformat = root.get("commitReformat").and_then(Json::as_bool) == Some(true);
        state.pdf_thumbnails_hidden = root
            .get("pdfThumbnailsHidden")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        state.markdown_toc_pinned =
            root.get("markdownTocPinned").and_then(Json::as_bool) == Some(true);
        state.vim_keys_noticed = root.get("vimKeysNoticed").and_then(Json::as_bool) == Some(true);
        state.recent_languages = (root.get("recentLanguages").and_then(Json::as_array))
            .unwrap_or_default()
            .iter()
            .filter_map(Json::as_str)
            .take(Self::RECENT_LANGUAGES)
            .map(str::to_owned)
            .collect();
        state.stay_outside_applications = root
            .get("stayOutsideApplications")
            .and_then(Json::as_bool)
            .unwrap_or(false);
        state.panel_width = root
            .get("panelWidth")
            .and_then(Json::as_number)
            .map(|width| width as f32);
        state.window = root
            .get("window")
            .and_then(Json::as_object)
            .and_then(window_frame);
        state.setup_pending = root.get("setupPending").and_then(Json::as_bool) == Some(true);
        state.version = root
            .get("version")
            .and_then(Json::as_str)
            .filter(|v| !v.is_empty())
            .map(str::to_owned);
        state.preview_modes = preview_modes(root.get("previewMode"));
        state.workspaces = root
            .get("workspaces")
            .map(workspace_set)
            .unwrap_or_default();
        for set in workspace_sets(root.get("otherWindows")) {
            if !state.others.contains(&set) && state.others.len() < MAX_SESSIONS {
                state.others.push(set);
            }
        }
        for path in paths(root.get("signOffRepos")) {
            if !state.sign_off_repos.contains(&path) {
                state.sign_off_repos.push(path);
            }
        }

        // 旧格式只记单个 `session`，即上次退出时的目录。
        if let Some(old) = root.get("session").and_then(session)
            && state.sessions.is_empty()
        {
            state.last = Some(old.folder.clone());
            state.sessions.push(old);
        }

        // 早先记在现场里的阅读位置排在后面：已有记录的文件以新的为准。
        state.places = read_file_places(root.get("places"));
        for (path, view) in earlier.take() {
            let known = state.places.iter().any(|(file, _)| *file == path);
            if !known && state.places.len() < MAX_PLACES {
                let view = Some(view);
                state.places.push((path, FilePlace { text: None, view }));
            }
        }
        Some(state)
    }
}
