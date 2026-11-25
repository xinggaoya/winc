//! 跨文件替换：全部替换的确认与执行。
//!
//! 未打开的文件直接写回磁盘且无法撤销，因此先弹确认框；确认后逐个文件替换，替换过的
//! 文件从结果里去掉，已经打开的文档就地改写。

use super::*;

impl App {
    /// 弹出确认框，用户确认后执行全部替换。
    ///
    /// 未打开的文件会直接写回磁盘且无法撤销，因此须先确认；确认后经事件循环进入
    /// [`App::replace_all_confirmed`]。搜索未完成、结果被截断或不全、没有结果时不弹出确认框，
    /// 改为在状态栏提示原因。
    pub(in crate::app) fn ask_replace_all(&mut self) {
        let g = &self.panels.global;

        if g.replacing.is_some() {
            self.flash(t!("workspace-replace-all-busy"));
        } else if g.due.is_some() || g.engine.is_running() {
            self.flash(t!("workspace-replace-all-searching"));
        } else if g.engine.is_truncated() {
            self.flash(t!(
                "workspace-replace-all-too-many",
                max = winc_editor::MAX_RESULTS
            ));
        } else if g.engine.gave_up() {
            self.flash(t!("editor-replace-all-gave-up"));
        } else if g.engine.remote_failed() {
            self.flash(t!("workspace-replace-all-remote-failed"));
        } else if g.engine.total() == 0 {
            self.flash(t!("workspace-replace-all-nothing"));
        } else {
            self.confirm_replace_all();
        }

        self.request_redraw();
    }

    /// 弹出全部替换的确认框。
    ///
    /// 主文本写明文件数、匹配数和替换文字，说明文本提示操作无法撤销。
    /// 「替换」为默认按钮（`↩`），「取消」对应 `Esc`。
    fn confirm_replace_all(&mut self) {
        let total = self.panels.global.engine.total();
        if self.window.handle.is_some() {
            use crate::app::AlertButton;
            let files = self.panels.global.engine.results.len();
            let replacement = self.panels.global.replace.text();
            let message = if replacement.is_empty() {
                t!(
                    "workspace-replace-all-confirm-delete",
                    files = files,
                    total = total
                )
            } else {
                // 替换文字完整显示、不截短，便于用户核对；过长时提示框自动折行。
                t!(
                    "workspace-replace-all-confirm",
                    files = files,
                    total = total,
                    replacement = replacement
                )
            };
            let buttons = vec![
                AlertButton::new(t!("workspace-replace-all-button")),
                AlertButton::cancel(t!("common-cancel")),
            ];
            let warning = t!("workspace-replace-all-warning");
            self.show_alert(&message, warning, buttons, None, move |index, _| {
                (index == 0).then_some(UserEvent::ReplaceAllConfirmed(total))
            });
            return;
        }

        // 没有窗口（测试中）时直接替换。
        self.replace_all_confirmed(total);
    }

    /// 处理确认框上的全部替换确认。
    ///
    /// 仅当结果与弹出确认框时相同（期间没有重新搜索、总处数不变）时替换；
    /// 否则在状态栏说明原因，不替换用户未确认的结果。
    pub(in crate::app) fn replace_all_confirmed(&mut self, total: usize) {
        let g = &self.panels.global;
        if g.due.is_some() || g.engine.is_running() || g.engine.total() != total {
            self.flash(t!("workspace-replace-all-changed"));
            return;
        }
        self.replace_all_global();
    }

    /// 替换结果列表中剩余的全部匹配，在状态栏提示结果，之后重新搜索。
    ///
    /// 已打开的文件在文档中替换，在界面线程完成；没有打开的文件需要逐个读取、写回磁盘，文件数可达
    /// 数千个，在后台完成，结束后由 [`App::poll_replace_all`] 提示结果。
    pub(in crate::app) fn replace_all_global(&mut self) {
        let Some(query) = self.panels.global.engine.query().cloned() else {
            return;
        };
        // 只替换结果列表中剩余的匹配；用户已从结果中移除的不替换。结果替换完就作废（最后重新
        // 搜索），直接取走，不复制；引擎随即清空，结果行跟着重排，即使之后没能重新搜索也不会
        // 指向已取走的结果。
        let expected = self.panels.global.engine.total();
        let files_left = std::mem::take(&mut self.panels.global.engine.results);
        self.panels.global.engine.clear();
        self.panels.global.rebuild_rows();
        let replacement = self.panels.global.replace.text().to_owned();
        self.panels.global.replace.remember();

        let mut tally = ReplaceTally::default();
        let mut closed = Vec::new();
        for file in files_left {
            match self.document_at(&file.path) {
                Some(doc) => {
                    let result = replace_in_document(doc, &query, &replacement, &file.matches)
                        .map_err(str::to_owned);
                    tally.add(&file.path, result, true);
                }
                None => closed.push(file),
            }
        }
        if closed.is_empty() {
            self.finish_replace_all(tally, expected);
            return;
        }

        let (sender, receiver) = mpsc::channel();
        self.panels.global.replacing = Some((receiver, expected));
        self.spawn_then(UserEvent::Replaced, move || {
            for file in &closed {
                let result = replace_in_file(&file.path, &query, &replacement, &file.matches);
                tally.add(&file.path, result, false);
            }
            let _ = sender.send(tally);
        });
    }

    /// 收取后台完成的替换，提示结果：单个文件的替换（[`App::replace_row`]）和跨文件替换。
    pub(in crate::app) fn poll_replace_all(&mut self) {
        let mut rows = Vec::new();
        self.panels
            .global
            .row_replacing
            .retain(|receiver| match receiver.try_recv() {
                Ok(done) => {
                    rows.push(done);
                    false
                }
                Err(mpsc::TryRecvError::Empty) => true,
                Err(mpsc::TryRecvError::Disconnected) => false,
            });
        for (path, result) in rows {
            self.row_replaced(path, result);
        }

        let Some((receiver, expected)) = &self.panels.global.replacing else {
            return;
        };
        let Ok(tally) = receiver.try_recv() else {
            return;
        };
        let expected = *expected;
        self.panels.global.replacing = None;
        self.finish_replace_all(tally, expected);
    }

    /// 在状态栏提示跨文件替换的结果 `tally`，之后重新搜索；`expected` 是替换前结果列表中的总处数。
    fn finish_replace_all(&mut self, tally: ReplaceTally, expected: usize) {
        let ReplaceTally {
            files,
            count,
            opened,
            failed,
        } = tally;
        let mut message = t!("workspace-replace-all-done", files = files, count = count);

        if opened > 0 {
            message.push_str(&t!("workspace-replace-all-unsaved", opened = opened));
        }

        let stale = expected.saturating_sub(count);
        if failed.is_empty() && stale > 0 {
            message.push_str(&t!("workspace-replace-all-stale", stale = stale));
        }

        if let Some(first) = failed.first() {
            message.push_str(&t!(
                "workspace-replace-all-failed",
                count = failed.len(),
                error = first.as_str()
            ));
        }

        self.flash(message);
        self.run_global_search();
    }

    /// 替换文件 `path` 中 `matches` 对应的匹配：已打开的文件改在文档里，没有打开的写回磁盘。
    ///
    /// 成功时返回替换的处数，以及文件是否已打开。
    ///
    /// # Errors
    ///
    /// 替换失败时（例如文件无法读写）返回失败原因。
    fn replace_in_path(
        &mut self,
        path: &Path,
        query: &Query,
        replacement: &str,
        matches: &[LineMatch],
    ) -> Result<(usize, bool), String> {
        match self.document_at(path) {
            Some(doc) => replace_in_document(doc, query, replacement, matches)
                .map(|n| (n, true))
                .map_err(str::to_owned),
            None => replace_in_file(path, query, replacement, matches).map(|n| (n, false)),
        }
    }

    /// 返回路径为 `path` 的已打开文档；尚未读取的不算，它的内容以磁盘为准。
    fn document_at(&mut self, path: &Path) -> Option<&mut Document> {
        self.editor
            .documents
            .iter_mut()
            .find(|d| d.path.as_deref() == Some(path) && !d.is_pending())
    }

    /// 返回有未保存修改的文档的路径和当前内容（rope 共享持有，不复制），项目搜索据此按内存中的
    /// 内容搜索这些文件。
    ///
    /// 未命名的文档不在其中。
    pub(in crate::app) fn dirty_ropes(&self) -> Vec<(PathBuf, ropey::Rope)> {
        self.editor
            .documents
            .iter()
            .filter(|d| d.is_dirty())
            .filter_map(|d| Some((d.path.clone()?, d.text().rope()?.clone())))
            .collect()
    }

    /// 返回路径为 `path`、有未保存修改的文档的当前内容；没有这样的文档时返回 `None`。
    pub(super) fn dirty_rope(&self, path: &Path) -> Option<ropey::Rope> {
        self.editor
            .documents
            .iter()
            .find(|d| d.path.as_deref() == Some(path))
            .filter(|d| d.is_dirty())
            .and_then(|d| d.text().rope().cloned())
    }

    /// 替换第 `index` 个结果行：目录行替换其中各个文件剩余的全部匹配，文件行替换该文件中剩余的
    /// 全部匹配，匹配行只替换这一处。
    pub(super) fn replace_row(&mut self, index: usize) {
        let Some(&row) = self.panels.global.rows().get(index) else {
            return;
        };
        let Some(query) = self.panels.global.engine.query().cloned() else {
            return;
        };
        let g = &self.panels.global;
        let targets: Vec<(PathBuf, Vec<LineMatch>, Option<usize>)> = match row {
            ResultRow::Match(f, m) => {
                let file = &g.engine.results[f];
                vec![(file.path.clone(), vec![file.matches[m].clone()], Some(m))]
            }
            ResultRow::Dir(_) | ResultRow::File(_) => (g.row_files(row).into_iter())
                .map(|f| &g.engine.results[f])
                .map(|file| (file.path.clone(), file.matches.clone(), None))
                .collect(),
        };
        let replacement = self.panels.global.replace.text().to_owned();
        for (path, matches, m) in targets {
            self.replace_matches((&query, &replacement), path, matches, m);
        }
        self.after_results_changed(index);
    }

    /// 替换文件 `path` 中的 `matches`：`m` 是只替换其中一处时它的序号。
    ///
    /// 已打开的文件在文档中替换。没有打开的文件需要读取、替换并写回磁盘（最大可达 4 GB），
    /// 在后台进行：这些匹配先从结果中移除，完成后由 [`App::row_replaced`] 提示结果，
    /// 并在后台重新搜索该文件。
    fn replace_matches(
        &mut self,
        (query, replacement): (&Query, &str),
        path: PathBuf,
        matches: Vec<LineMatch>,
        m: Option<usize>,
    ) {
        if self.document_at(&path).is_none() {
            match m {
                None => self.panels.global.engine.remove_file(&path),
                Some(m) => self.panels.global.engine.remove_match(&path, m),
            }
            let (sender, receiver) = mpsc::channel();
            self.panels.global.row_replacing.push(receiver);
            let (query, replacement) = (query.clone(), replacement.to_owned());
            self.spawn_then(UserEvent::Replaced, move || {
                let result = replace_in_file(&path, &query, &replacement, &matches);
                let _ = sender.send((path, result));
            });
            return;
        }
        let result = self
            .replace_in_path(&path, query, replacement, &matches)
            .map(|(n, _)| n > 0);

        match result {
            Ok(true) => {}
            Ok(false) => self.flash(t!("workspace-replace-stale")),
            Err(e) => self.notify(
                NoticeKind::Error,
                t!("workspace-replace-failed", error = e.to_string()),
            ),
        }

        // 只重新搜索这一个文件，避免结果列表被整个清空再重建。
        let rope = self.dirty_rope(&path);
        self.panels.global.engine.refresh_file(&path, rope.as_ref());
    }

    /// 后台替换完没有打开的文件 `path`：替换失败或匹配已经过时时提示，之后在后台重新搜索该文件。
    fn row_replaced(&mut self, path: PathBuf, result: Result<usize, String>) {
        match result {
            Ok(0) => self.flash(t!("workspace-replace-stale")),
            Ok(_) => {}
            Err(e) => self.notify(NoticeKind::Error, t!("workspace-replace-failed", error = e)),
        }
        self.panels.global.engine.refresh_files([(path, None)]);
        self.request_redraw();
    }
}
