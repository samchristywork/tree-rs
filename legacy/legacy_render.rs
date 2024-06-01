fn read_dir_recursive_and_print(dirname: PathBuf, indent: &Vec<String>) {
    if indent.len() == 0 {
        print_node_name(&dirname);
    }

    let entries = match std::fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    entries.retain(|entry| {
        let entry = entry.as_ref().unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            return false;
        }
        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            return false;
        }

        true
    });

    for (i, entry) in entries.iter().enumerate() {
        let entry = entry.as_ref().unwrap();
        let path = entry.path();

        if path.is_dir() {
            if i == entries.len() - 1 {
                print!("{}└── ", indent.join(""));
                print_node_name(&path);
                let mut indent = indent.clone();
                indent.push("    ".to_string());
                read_dir_recursive_and_print(path, &indent);
            } else {
                print!("{}├── ", indent.join(""));
                print_node_name(&path);
                let mut indent = indent.clone();
                indent.push("│   ".to_string());
                read_dir_recursive_and_print(path, &indent);
            }
        } else {
            if i == entries.len() - 1 {
                print!("{}└── ", indent.join(""));
                print_node_name(&path);
            } else {
                print!("{}├── ", indent.join(""));
                print_node_name(&path);
            }
        }
    }
}

fn read_dir_recursive(dirname: PathBuf) -> TreeNode {
    let mut root = TreeNode {
        color: 33,
        val: dirname.file_name().unwrap().to_str().unwrap().to_string(),
        children: Vec::new(),
        node_type: NodeType::Dir,
    };

    let entries = match std::fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return root;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            continue;
        }

        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            continue;
        }

        if path.is_dir() {
            let child = read_dir_recursive(path);
            root.children.push(child);
        } else {
            let child = TreeNode {
                color: 34,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::File,
            };
            root.children.push(child);
        }
    }

    root
}

fn read_dir_recursive_in_place(root: &mut TreeNode, dirname: PathBuf) {
    root.color = 33;
    root.val = dirname.file_name().unwrap().to_str().unwrap().to_string();
    root.children = Vec::new();

    let entries = match std::fs::read_dir(dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            continue;
        }

        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            continue;
        }

        if path.is_dir() {
            let mut child = TreeNode {
                color: 33,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::Dir,
            };
            read_dir_recursive_in_place(&mut child, path);
            root.children.push(child);
        } else {
            let child = TreeNode {
                color: 34,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::File,
            };
            root.children.push(child);
        }
    }
}

fn legacy_read_dir_incremental(
    root: &mut TreeNode,
    dirname: PathBuf,
    begin_from: Option<PathBuf>,
    counter: &mut i32,
    increment: i32,
    directories: &mut i32,
    files: &mut i32,
) -> Option<PathBuf> {
    root.color = 33;
    root.val = dirname.file_name().unwrap().to_str().unwrap().to_string();

    let entries = match std::fs::read_dir(&dirname) {
        Ok(entries) => entries,
        Err(_) => {
            return None;
        }
    };

    let mut entries: Vec<_> = entries.collect();
    entries.sort_by(|a, b| a.as_ref().unwrap().path().cmp(&b.as_ref().unwrap().path()));

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.file_name().unwrap().to_str().unwrap().starts_with(".") {
            continue;
        }

        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("target")
        {
            continue;
        }

        if let Some(bf) = &begin_from {
            let bf = bf.iter().zip(path.iter()).collect::<Vec<_>>();
            if bf.last().unwrap().0 > bf.last().unwrap().1 {
                continue;
            }
        }

        if *counter > increment {
            return Some(path);
        }

        if path.is_dir() {
            let val = path.file_name().unwrap().to_str().unwrap().to_string();
            let last = root.children.last_mut();
            match last {
                Some(last) => {
                    if last.val == val {
                        let left_off_from = legacy_read_dir_incremental(
                            last,
                            path,
                            begin_from.clone(),
                            counter,
                            increment,
                            directories,
                            files,
                        );
                        if let Some(left_off_from) = left_off_from {
                            return Some(left_off_from);
                        }
                    } else {
                        *directories += 1;
                        let mut child = TreeNode {
                            color: 33,
                            val,
                            children: Vec::new(),
                            node_type: NodeType::Dir,
                        };
                        let left_off_from = legacy_read_dir_incremental(
                            &mut child,
                            path,
                            begin_from.clone(),
                            counter,
                            increment,
                            directories,
                            files,
                        );
                        root.children.push(child);
                        if let Some(left_off_from) = left_off_from {
                            return Some(left_off_from);
                        }
                    }
                }
                None => {
                    *counter += 1;
                    *directories += 1;
                    let mut child = TreeNode {
                        color: 33,
                        val,
                        children: Vec::new(),
                        node_type: NodeType::Dir,
                    };
                    let left_off_from = legacy_read_dir_incremental(
                        &mut child,
                        path,
                        begin_from.clone(),
                        counter,
                        increment,
                        directories,
                        files,
                    );
                    root.children.push(child);
                    if let Some(left_off_from) = left_off_from {
                        return Some(left_off_from);
                    }
                }
            }
        } else {
            *counter += 1;
            *files += 1;
            let child = TreeNode {
                color: 34,
                val: path.file_name().unwrap().to_str().unwrap().to_string(),
                children: Vec::new(),
                node_type: NodeType::File,
            };
            root.children.push(child);
        }
    }

    None
}

async fn legacy_render(root: &mut TreeNode, dirname: PathBuf) {
    let mut terminal = term_setup();

    let content = print_tree(&root, &Vec::new(), &ColorOptions::NoColor);
    terminal.draw(|f| ui(f, None, Some(content))).unwrap();

    let mut search_term = String::new();

    let (tick_tx, tick_rx) = mpsc::channel(1);
    let (tx, mut rx) = mpsc::channel(100);
    task::spawn(async move {
        loop {
            if let Ok(event) = event::read() {
                if tx.send(event).await.is_err() {
                    break;
                }
            }
        }
    });

    let mut interval = tokio::time::interval(Duration::from_millis(10));
    task::spawn(async move {
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if tick_tx.send(()).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });

    let mut tick_rx = Some(tick_rx);

    let mut ret = None;
    let mut running = true;
    loop {
        tokio::select! {
            _ = tick_rx.as_mut().unwrap().recv() => {
                let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                loop {
                    if running{
                    let mut counter = 0;
                    ret = legacy_read_dir_incremental(root, dirname.clone(), ret, &mut counter, 4, &mut 0, &mut 0);

                    let tree = filter_tree(&root, &search_term);
                    let content = print_tree(&tree, &Vec::new(), &ColorOptions::NoColor);
                    terminal
                        .draw(|f| ui(f, Some(search_term.clone()), Some(content.clone())))
                        .unwrap();

                    if ret.is_none() {
                        running = false;
                    }
                    }
                    let end = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                    if end - start > 100 {
                        break;
                    }
                }
            }
            Some(event) = rx.recv() => {
                if let Event::Key(key) = event {
                    match key.code {
                        KeyCode::Char(c) => {
                            search_term.push(c);
                            refresh(&root, search_term.clone(), &mut terminal);
                            }
                        KeyCode::Esc => {
                            break;
                        }
                        KeyCode::Backspace => {
                            search_term.pop();
                            refresh(&root, search_term.clone(), &mut terminal);
                            }
                        _ => {
                        }
                    }
                }
            }
        }
    }

    term_teardown(&mut terminal);
}
