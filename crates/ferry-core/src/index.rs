use crate::lexer::{lex, Token, TokenKind};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct TypeInfo {
    pub name: String,
    pub(crate) fqn: String,
    pub(crate) parent_fqn: Option<String>,
    pub(crate) related_fqns: Vec<String>,
    pub(crate) trait_fqns: Vec<String>,
    pub is_trait: bool,
    pub parent: Option<String>,
    pub interfaces: Vec<String>,
    pub traits: Vec<String>,
    pub props: HashSet<String>,
    pub private_props: HashSet<String>,
    pub(crate) static_props: HashSet<String>,
    pub methods: HashSet<String>,
    pub(crate) signatures: HashMap<String, MethodSig>,
    pub open: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MethodSig {
    pub is_static: bool,
    pub is_private: bool,
    pub is_abstract: bool,
    pub params: Vec<crate::analyzer::ParamShape>,
    pub returns: Option<Vec<String>>,
    pub returns_shown: Option<Vec<String>>,
    pub param_fqns: Vec<Vec<String>>,
    pub return_fqns: Vec<String>,
}

pub(crate) fn resolve_type(tokens: &[Token], at: usize, names: &Names) -> String {
    let lower = tokens[at].text.trim_start_matches('\\').to_ascii_lowercase();
    let builtin = [
        "int", "float", "string", "bool", "array", "null", "callable", "iterable", "false", "true", "void", "mixed", "object", "self",
        "static", "parent", "never",
    ];
    if builtin.contains(&lower.as_str()) {
        lower
    } else {
        names.resolve(at, &tokens[at].text)
    }
}

pub(crate) fn method_sig(tokens: &[Token], function: usize, close_body_scan: usize, names: &Names) -> Option<(String, MethodSig)> {
    let name_at = (function + 1..close_body_scan).find(|&j| !tokens[j].is_punct("&"))?;
    let name = tokens.get(name_at).filter(|t| t.kind == TokenKind::Ident)?;
    let open = name_at + 1;
    if !tokens.get(open)?.is_punct("(") {
        return None;
    }
    let mut depth = 0;
    let close = (open..tokens.len()).find(|&j| {
        if tokens[j].is_punct("(") {
            depth += 1;
        } else if tokens[j].is_punct(")") {
            depth -= 1;
        }
        depth == 0
    })?;
    let start =
        (0..function).rev().find(|&j| tokens[j].is_punct(";") || tokens[j].is_punct("{") || tokens[j].is_punct("}")).map_or(0, |j| j + 1);
    let modifier = |m: &str| tokens[start..function].iter().any(|t| t.is_ident_ci(m));
    let params = crate::analyzer::param_shapes(tokens, open, close);
    let sig = MethodSig {
        is_static: modifier("static"),
        is_private: modifier("private"),
        is_abstract: modifier("abstract"),
        param_fqns: params.iter().map(|p| p.ty_at.iter().map(|&k| resolve_type(tokens, k, names)).collect()).collect(),
        return_fqns: crate::analyzer::return_type_at(tokens, close).into_iter().map(|k| resolve_type(tokens, k, names)).collect(),
        params,
        returns: crate::analyzer::return_type(tokens, close),
        returns_shown: crate::analyzer::return_type_shown(tokens, close),
    };
    Some((name.text.to_ascii_lowercase(), sig))
}

pub(crate) struct TypeDecl {
    pub open: usize,
    pub close: usize,
    pub info: TypeInfo,
}

#[derive(Debug, Default)]
pub struct ProjectIndex {
    pub framework: Option<crate::project::Framework>,
    functions: HashSet<String>,
    function_required: HashMap<String, usize>,
    version_includes: HashSet<String>,
    dead_includes: HashSet<String>,
    types: HashMap<String, Vec<TypeInfo>>,
    interfaces: HashMap<String, Vec<InterfaceInfo>>,
    constants: HashSet<String>,
    mysqli_report: bool,
    enums: HashSet<String>,
    target: Option<crate::version::PhpVersion>,
}

#[derive(Debug, Clone)]
struct InterfaceInfo {
    name: String,
    fqn: String,
    parent_fqns: Vec<String>,
    parents: Vec<String>,
    declares_methods: bool,
    signatures: HashMap<String, MethodSig>,
    constants: HashSet<String>,
}

impl ProjectIndex {
    pub fn build(root: &Path, files: &[PathBuf]) -> Self {
        Self::build_for(root, files, None)
    }

    pub fn build_for(root: &Path, files: &[PathBuf], target: Option<crate::version::PhpVersion>) -> Self {
        let mut index = Self { framework: Some(crate::project::detect_framework(root)), target, ..Self::default() };
        let mut seen = HashSet::new();
        let mut declared = Vec::new();
        for path in files.iter().cloned().chain(composer_files(root)) {
            if !seen.insert(std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone())) {
                continue;
            }
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > crate::project::MAX_FILE_BYTES) {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                let added = index.add(&lex(&String::from_utf8_lossy(&bytes)).tokens);
                let base = path.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
                declared.push((base, added));
            }
        }
        for (base, names) in declared {
            if index.dead_includes.contains(&base) {
                for name in names {
                    index.functions.remove(&name);
                    index.function_required.remove(&name);
                }
            }
        }
        index
    }

    pub fn included_by_version_check(&self, file: &str) -> bool {
        let base = file.rsplit(['/', '\\']).next().unwrap_or(file).to_ascii_lowercase();
        self.version_includes.contains(&base)
    }

    pub fn from_tokens(tokens: &[Token]) -> Self {
        Self::from_tokens_for(tokens, None)
    }

    pub fn from_tokens_for(tokens: &[Token], target: Option<crate::version::PhpVersion>) -> Self {
        let mut index = Self { target, ..Self::default() };
        index.add(tokens);
        index
    }

    pub(crate) fn interface_declares_constant(&self, fqn: &str, name: &str) -> bool {
        let key = short(fqn);
        let found = self.interfaces.get(&key).and_then(|v| v.iter().find(|i| i.fqn == fqn));
        found.is_some_and(|i| {
            i.constants.contains(name) || i.parent_fqns.iter().any(|p| p != fqn && self.interface_declares_constant(p, name))
        })
    }

    pub(crate) fn is_enum(&self, fqn: &str) -> bool {
        self.enums.contains(fqn)
    }

    /// Whether a known subclass inherits this method instead of overriding it.
    pub(crate) fn inherits_unchanged(&self, class: &str, method: &str) -> bool {
        let short = |name: &str| name.rsplit('\\').next().unwrap_or(name).to_ascii_lowercase();
        let (class, method) = (short(class), method.to_ascii_lowercase());
        self.types.values().flatten().any(|t| t.parent.as_deref().is_some_and(|p| short(p) == class) && !t.methods.contains(&method))
    }

    pub(crate) fn sets_mysqli_report(&self) -> bool {
        self.mysqli_report
    }

    pub(crate) fn declares_type(&self, short: &str) -> bool {
        self.types.contains_key(short) || self.interfaces.contains_key(short)
    }

    pub fn defines_constant(&self, name: &str) -> bool {
        self.constants.contains(name)
    }

    fn add(&mut self, tokens: &[Token]) -> Vec<String> {
        let mut added = Vec::new();
        self.constants.extend(crate::analyzer::defined_constants(tokens));
        self.mysqli_report |= tokens.iter().any(|t| t.is_ident_ci("mysqli_report") || t.is_ident_ci("report_mode"));
        let methods = crate::analyzer::method_tokens(tokens);
        let spaces = namespaces(tokens);
        let names = Names::new(tokens);
        let dead = self.target.map(|t| crate::guards::dead_spans(tokens, t)).unwrap_or_default();
        for (i, w) in tokens.windows(2).enumerate() {
            if w[0].is_ident_ci("enum") && w[1].kind == TokenKind::Ident {
                self.enums.insert(names.declared(i, &w[1].text));
            }
        }
        for (i, t) in tokens.iter().enumerate() {
            if !t.is_ident_ci("function") || methods.contains(&i) {
                continue;
            }
            if dead.iter().any(|&(start, end)| (t.line, t.column) >= start && (t.line, t.column) <= end) {
                continue;
            }
            let name = tokens[i + 1..].iter().find(|t| !t.is_punct("&"));
            if let Some(name) = name.filter(|n| n.kind == TokenKind::Ident) {
                let qualified = qualify(namespace_at(&spaces, i), &name.text);
                let required = method_sig(tokens, i, tokens.len(), &names)
                    .map_or(0, |(_, sig)| sig.params.iter().rposition(|p| !p.optional && !p.variadic).map_or(0, |k| k + 1));
                let known = self.functions.contains(&qualified);
                let entry = self.function_required.entry(qualified.clone()).or_insert(required);
                if known {
                    *entry = (*entry).min(required);
                }
                if self.functions.insert(qualified.clone()) {
                    added.push(qualified);
                }
            }
        }
        self.version_includes.extend(version_includes(tokens));
        self.dead_includes.extend(dead_includes(tokens, &dead));
        for decl in type_decls(tokens) {
            let name = decl.name;
            self.types.entry(name).or_default().push(decl.decl.info);
        }
        for i in 0..tokens.len() {
            if !tokens[i].is_ident_ci("interface") || i > 0 && tokens[i - 1].is_punct("::") {
                continue;
            }
            let Some(name) = tokens.get(i + 1).filter(|n| n.kind == TokenKind::Ident) else { continue };
            let Some(open) = (i + 1..tokens.len()).find(|&k| tokens[k].is_punct("{") || tokens[k].is_punct(";")) else { continue };
            if !tokens[open].is_punct("{") {
                continue;
            }
            let parent_tokens: Vec<&Token> =
                tokens[i + 2..open].iter().filter(|t| t.kind == TokenKind::Ident && !t.is_ident_ci("extends")).collect();
            let parents = parent_tokens.iter().map(|t| short(&t.text)).collect();
            let parent_fqns = parent_tokens.iter().map(|t| names.resolve(i, &t.text)).collect();
            let mut depth = 0usize;
            let body_end = (open..tokens.len())
                .find(|&k| {
                    if tokens[k].is_punct("{") {
                        depth += 1;
                    } else if tokens[k].is_punct("}") {
                        depth -= 1;
                    }
                    depth == 0
                })
                .unwrap_or(tokens.len() - 1);
            let signatures: HashMap<String, MethodSig> = (open..body_end)
                .filter(|&k| tokens[k].is_ident_ci("function"))
                .filter_map(|k| method_sig(tokens, k, body_end, &names))
                .collect();
            let declares_methods = !signatures.is_empty() || tokens[open..body_end].iter().any(|t| t.is_ident_ci("function"));
            let constants = (open..body_end)
                .filter(|&k| tokens[k].is_ident_ci("const"))
                .filter_map(|k| (k + 1..body_end).find(|&j| tokens[j].is_punct("=")))
                .map(|eq| tokens[eq - 1].text.clone())
                .collect();
            self.interfaces.entry(short(&name.text)).or_default().push(InterfaceInfo {
                name: name.text.clone(),
                fqn: qualify(namespace_at(&spaces, i), &name.text),
                parent_fqns,
                parents,
                declares_methods,
                signatures,
                constants,
            });
        }
        added
    }

    pub(crate) fn instance_method(&self, fqn: &str, method: &str) -> Option<bool> {
        let mut current = fqn.to_string();
        for _ in 0..32 {
            let t = self.type_by_fqn(&current)?;
            if t.methods.contains("__callstatic") {
                return None;
            }
            if let Some(sig) = t.signatures.get(method) {
                return Some(!sig.is_static);
            }
            for tr in &t.trait_fqns {
                let tr = self.type_by_fqn(tr)?;
                if let Some(sig) = tr.signatures.get(method) {
                    return Some(!sig.is_static);
                }
            }
            current = t.parent_fqn.clone()?;
        }
        None
    }

    pub(crate) fn method_signature(&self, fqn: &str, method: &str) -> Option<&MethodSig> {
        let mut current = fqn.to_string();
        for _ in 0..32 {
            let t = self.type_by_fqn(&current)?;
            if let Some(sig) = t.signatures.get(method) {
                return Some(sig);
            }
            for tr in &t.trait_fqns {
                if let Some(sig) = self.type_by_fqn(tr)?.signatures.get(method) {
                    return Some(sig);
                }
            }
            current = t.parent_fqn.clone()?;
        }
        None
    }

    pub(crate) fn extends_or_is(&self, child: &str, ancestor: &str) -> Option<bool> {
        let mut current = child.to_string();
        for _ in 0..32 {
            if current.eq_ignore_ascii_case(ancestor) {
                return Some(true);
            }
            match self.type_by_fqn(&current)?.parent_fqn.clone() {
                Some(p) => current = p,
                None => return Some(false),
            }
        }
        None
    }

    pub(crate) fn class(&self, name: &str) -> Option<&TypeInfo> {
        self.unique(name).filter(|t| !t.is_trait)
    }

    pub(crate) fn class_or_trait(&self, name: &str) -> Option<&TypeInfo> {
        self.unique(name)
    }

    pub(crate) fn marker_interface_parents(&self, name: &str) -> Option<&[String]> {
        match self.interfaces.get(name).map(Vec::as_slice) {
            Some([only]) if !only.declares_methods => Some(&only.parents),
            _ => None,
        }
    }

    pub(crate) fn required_args(&self, namespace: &str, name: &str, rooted: bool) -> Option<usize> {
        let name = name.trim_start_matches('\\').to_ascii_lowercase();
        if !rooted && !namespace.is_empty() {
            if let Some(&n) = self.function_required.get(&qualify(namespace, &name)) {
                return Some(n);
            }
        }
        self.function_required.get(&name).copied()
    }

    pub(crate) fn declares_function(&self, namespace: &str, name: &str, rooted: bool) -> bool {
        let name = name.to_ascii_lowercase();
        self.functions.contains(&name) || (!rooted && !namespace.is_empty() && self.functions.contains(&qualify(namespace, &name)))
    }

    pub(crate) fn type_by_fqn(&self, fqn: &str) -> Option<&TypeInfo> {
        let key = short(fqn);
        if self.interfaces.get(&key).is_some_and(|v| v.iter().any(|i| i.fqn == fqn)) {
            return None;
        }
        match self.types.get(&key)?.iter().filter(|t| t.fqn == fqn).collect::<Vec<_>>().as_slice() {
            [only] => Some(only),
            _ => None,
        }
    }

    pub(crate) fn supertypes(&self, fqn: &str) -> Option<Vec<String>> {
        if let Some(t) = self.type_by_fqn(fqn) {
            return Some(t.parent_fqn.iter().chain(&t.related_fqns).cloned().collect());
        }
        self.interface_by_fqn(fqn).map(|(_, _, parents)| parents.to_vec())
    }

    pub(crate) fn interface_by_fqn(&self, fqn: &str) -> Option<(&str, &HashMap<String, MethodSig>, &[String])> {
        let key = short(fqn);
        if self.types.get(&key).is_some_and(|v| v.iter().any(|t| t.fqn == fqn)) {
            return None;
        }
        match self.interfaces.get(&key)?.iter().filter(|i| i.fqn == fqn).collect::<Vec<_>>().as_slice() {
            [only] => Some((only.name.as_str(), &only.signatures, &only.parent_fqns)),
            _ => None,
        }
    }

    fn unique(&self, name: &str) -> Option<&TypeInfo> {
        match self.types.get(name).map(Vec::as_slice) {
            Some([only]) => Some(only),
            _ => None,
        }
    }

    pub(crate) fn properties(&self, info: &TypeInfo) -> Option<HashSet<String>> {
        self.collect(info, 0).map(|(all, _)| all)
    }

    fn collect(&self, info: &TypeInfo, depth: usize) -> Option<(HashSet<String>, HashSet<String>)> {
        if info.open || depth > 32 {
            return None;
        }
        let mut all: HashSet<String> = info.props.difference(&info.static_props).cloned().collect();
        let mut private = info.private_props.clone();
        for t in &info.traits {
            let (props, hidden) = self.collect(self.unique(t)?, depth + 1)?;
            all.extend(props);
            private.extend(hidden);
        }
        if let Some(parent) = &info.parent {
            if parent == "stdclass" {
                return None;
            }
            let (props, hidden) = self.collect(self.unique(parent)?, depth + 1)?;
            all.extend(props.difference(&hidden).cloned());
        }
        Some((all, private))
    }
}

fn qualify(namespace: &str, name: &str) -> String {
    let name = name.trim_start_matches('\\').to_ascii_lowercase();
    if namespace.is_empty() {
        name
    } else {
        format!("{}\\{name}", namespace.to_ascii_lowercase())
    }
}

fn short(name: &str) -> String {
    name.rsplit('\\').next().unwrap_or_default().to_ascii_lowercase()
}

pub(crate) fn namespace_section(spaces: &[(usize, String)], at: usize) -> usize {
    spaces.iter().rev().find(|(start, _)| *start <= at).map_or(0, |(start, _)| *start)
}

pub(crate) fn namespaces(tokens: &[Token]) -> Vec<(usize, String)> {
    let mut out = vec![(0, String::new())];
    for (i, t) in tokens.iter().enumerate() {
        if !t.is_ident_ci("namespace") || i > 0 && tokens[i - 1].is_punct("::") {
            continue;
        }
        match tokens.get(i + 1) {
            Some(n) if n.kind == TokenKind::Ident => out.push((i, n.text.trim_start_matches('\\').to_string())),
            Some(n) if n.is_punct("{") => out.push((i, String::new())),
            _ => {}
        }
    }
    out
}

pub(crate) struct Names {
    spaces: Vec<(usize, String)>,
    uses: Vec<(usize, String, String)>,
}

impl Names {
    pub(crate) fn new(tokens: &[Token]) -> Self {
        let spaces = namespaces(tokens);
        let mut uses = Vec::new();
        let mut stack: Vec<bool> = Vec::new();
        for i in 0..tokens.len() {
            let t = &tokens[i];
            if t.is_punct("{") {
                let ns = i > 0 && (tokens[i - 1].is_ident_ci("namespace") || i > 1 && tokens[i - 2].is_ident_ci("namespace"));
                stack.push(ns);
            } else if t.is_punct("}") {
                stack.pop();
            } else if t.is_ident_ci("use") && stack.iter().all(|ns| *ns) && !(i > 0 && tokens[i - 1].is_punct(")")) {
                let end = (i + 1..tokens.len()).find(|&k| tokens[k].is_punct(";") || tokens[k].is_punct("{")).unwrap_or(tokens.len());
                if end == tokens.len()
                    || tokens[end].is_punct("{")
                    || tokens.get(i + 1).is_some_and(|n| n.is_ident_ci("function") || n.is_ident_ci("const"))
                {
                    continue;
                }
                for part in tokens[i + 1..end].split(|t| t.is_punct(",")) {
                    let idents: Vec<&Token> = part.iter().filter(|t| t.kind == TokenKind::Ident).collect();
                    let (target, alias) = match idents.as_slice() {
                        [target] => (target, short(&target.text)),
                        [target, kw, alias] if kw.is_ident_ci("as") => (target, alias.text.to_ascii_lowercase()),
                        _ => continue,
                    };
                    uses.push((i, alias, target.text.trim_start_matches('\\').to_ascii_lowercase()));
                }
            }
        }
        Names { spaces, uses }
    }

    pub(crate) fn declared(&self, at: usize, name: &str) -> String {
        qualify(namespace_at(&self.spaces, at), name)
    }

    pub(crate) fn resolve(&self, at: usize, name: &str) -> String {
        if let Some(rooted) = name.strip_prefix('\\') {
            return rooted.to_ascii_lowercase();
        }
        let ns = namespace_at(&self.spaces, at);
        let lower = name.to_ascii_lowercase();
        let (first, rest) = lower.split_once('\\').map_or((lower.as_str(), None), |(f, r)| (f, Some(r)));
        let alias = self.uses.iter().rev().find(|(pos, alias, _)| *pos < at && alias == first && namespace_at(&self.spaces, *pos) == ns);
        match (alias, rest) {
            (Some((_, _, target)), Some(rest)) => format!("{target}\\{rest}"),
            (Some((_, _, target)), None) => target.clone(),
            _ => qualify(ns, name),
        }
    }
}

pub(crate) fn namespace_at(spaces: &[(usize, String)], i: usize) -> &str {
    let k = spaces.partition_point(|(at, _)| *at <= i);
    spaces[k.saturating_sub(1)].1.as_str()
}

struct NamedDecl {
    name: String,
    decl: TypeDecl,
}

fn type_decls(tokens: &[Token]) -> Vec<NamedDecl> {
    let mut out = Vec::new();
    let names = Names::new(tokens);
    for i in 0..tokens.len() {
        let t = &tokens[i];
        if !(t.is_ident_ci("class") || t.is_ident_ci("trait"))
            || i > 0 && (tokens[i - 1].is_punct("::") || tokens[i - 1].is_ident_ci("new"))
        {
            continue;
        }
        let Some(name) = tokens.get(i + 1).filter(|n| n.kind == TokenKind::Ident) else { continue };
        let Some(mut decl) = type_decl(tokens, i, &names) else { continue };
        decl.info.fqn = qualify(namespace_at(&names.spaces, i), &name.text);
        decl.info.parent_fqn = decl_header_name(tokens, i, decl.open, "extends").map(|n| names.resolve(i, n));
        decl.info.related_fqns = decl_header_list(tokens, i, decl.open, "implements").iter().map(|n| names.resolve(i, n)).collect();
        decl.info.trait_fqns = decl_trait_names(tokens, decl.open, decl.close).iter().map(|&(at, n)| names.resolve(at, n)).collect();
        out.push(NamedDecl { name: short(&name.text), decl });
    }
    out
}

fn decl_header_name<'a>(tokens: &'a [Token], kw: usize, open: usize, keyword: &str) -> Option<&'a str> {
    let at = (kw..open).find(|&k| tokens[k].is_ident_ci(keyword))?;
    tokens.get(at + 1).filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.as_str())
}

fn decl_header_list<'a>(tokens: &'a [Token], kw: usize, open: usize, keyword: &str) -> Vec<&'a str> {
    let Some(at) = (kw..open).find(|&k| tokens[k].is_ident_ci(keyword)) else { return Vec::new() };
    tokens[at + 1..open].iter().filter(|t| t.kind == TokenKind::Ident).map(|t| t.text.as_str()).collect()
}

fn decl_trait_names(tokens: &[Token], open: usize, close: usize) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut k = open;
    while k < close {
        let t = &tokens[k];
        if t.is_punct("{") || t.is_punct("(") {
            depth += 1;
        } else if t.is_punct("}") || t.is_punct(")") {
            depth -= 1;
        } else if depth == 1 && t.is_ident_ci("use") {
            while let Some(n) = tokens.get(k + 1).filter(|n| !n.is_punct(";") && !n.is_punct("{")) {
                if n.kind == TokenKind::Ident {
                    out.push((k + 1, n.text.as_str()));
                }
                k += 1;
            }
        }
        k += 1;
    }
    out
}

pub(crate) fn class_and_trait_decls(tokens: &[Token]) -> Vec<TypeDecl> {
    type_decls(tokens).into_iter().map(|d| d.decl).collect()
}

pub(crate) fn class_decls(tokens: &[Token]) -> Vec<TypeDecl> {
    type_decls(tokens).into_iter().map(|d| d.decl).filter(|d| !d.info.is_trait).collect()
}

fn type_decl(tokens: &[Token], kw: usize, names: &Names) -> Option<TypeDecl> {
    let open = (kw + 1..tokens.len()).find(|&k| tokens[k].is_punct("{") || tokens[k].is_punct(";"))?;
    if !tokens[open].is_punct("{") {
        return None;
    }
    let mut depth = 0usize;
    let close = (open..tokens.len()).find(|&k| {
        if tokens[k].is_punct("{") {
            depth += 1;
        } else if tokens[k].is_punct("}") {
            depth -= 1;
        }
        depth == 0
    })?;
    let header = &tokens[kw..open];
    let parent = header
        .iter()
        .position(|t| t.is_ident_ci("extends"))
        .and_then(|p| header.get(p + 1))
        .filter(|t| t.kind == TokenKind::Ident)
        .map(|t| short(&t.text));
    let attr_start = (0..kw).rev().find(|&k| tokens[k].is_punct(";") || tokens[k].is_punct("}") || tokens[k].is_punct("{")).unwrap_or(0);
    let mut open_class =
        tokens[attr_start..kw].iter().any(|t| t.text.trim_start_matches('\\').eq_ignore_ascii_case("AllowDynamicProperties"));
    let modifiers = ["public", "protected", "private", "readonly", "var"];
    let interfaces = header
        .iter()
        .position(|t| t.is_ident_ci("implements"))
        .map(|p| header[p + 1..].iter().filter(|t| t.kind == TokenKind::Ident).map(|t| short(&t.text)).collect())
        .unwrap_or_default();
    let name = tokens.get(kw + 1).map(|t| t.text.clone()).unwrap_or_default();
    let mut info = TypeInfo { name, is_trait: tokens[kw].is_ident_ci("trait"), parent, interfaces, ..TypeInfo::default() };
    let mut depth = 0usize;
    let mut k = open;
    while k < close {
        let t = &tokens[k];
        if t.is_punct("{") || t.is_punct("(") {
            depth += 1;
        } else if t.is_punct("}") || t.is_punct(")") {
            depth -= 1;
        } else if depth == 1 && t.is_ident_ci("use") {
            while let Some(n) = tokens.get(k + 1).filter(|n| !n.is_punct(";") && !n.is_punct("{")) {
                if n.kind == TokenKind::Ident {
                    info.traits.push(short(&n.text));
                }
                k += 1;
            }
        } else if depth == 1 && t.is_ident_ci("function") {
            let name = tokens[k + 1..].iter().find(|n| !n.is_punct("&"));
            if let Some(name) = name.filter(|n| n.kind == TokenKind::Ident) {
                open_class |= name.is_ident_ci("__set");
                info.methods.insert(name.text.to_ascii_lowercase());
            }
            if let Some((name, sig)) = method_sig(tokens, k, close, names) {
                info.signatures.insert(name, sig);
            }
        } else if t.kind == TokenKind::Variable {
            let promoted = depth == 2
                && (0..k)
                    .rev()
                    .take_while(|&j| !tokens[j].is_punct("(") && !tokens[j].is_punct(","))
                    .any(|j| modifiers.iter().any(|m| tokens[j].is_ident_ci(m)));
            if depth == 1 || promoted {
                let stops: &[&str] = if depth == 1 { &["{", "}", ";"] } else { &["(", ","] };
                let start = (0..k).rev().find(|&j| stops.iter().any(|p| tokens[j].is_punct(p))).map_or(0, |j| j + 1);
                if tokens[start..k].iter().any(|m| m.is_ident_ci("private")) {
                    info.private_props.insert(t.text.clone());
                }
                if depth == 1 && tokens[start..k].iter().any(|m| m.is_ident_ci("static")) {
                    info.static_props.insert(t.text.clone());
                }
                info.props.insert(t.text.clone());
            }
        }
        k += 1;
    }
    info.open = open_class;
    Some(TypeDecl { open, close, info })
}

pub(crate) fn composer_files(root: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(root.join("vendor/composer/autoload_files.php")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (marker, base) in [("$vendorDir . '", root.join("vendor")), ("$baseDir . '", root.to_path_buf())] {
        for (at, _) in text.match_indices(marker) {
            let rest = &text[at + marker.len()..];
            if let Some(end) = rest.find('\'') {
                let rel = rest[..end].trim_start_matches('/');
                if !rel.contains("..") {
                    out.push(base.join(rel));
                }
            }
        }
    }
    out
}

const VERSION_CHECKS: [&str; 7] =
    ["php_version_id", "php_version", "php_major_version", "version_compare", "function_exists", "class_exists", "interface_exists"];

fn version_condition(tokens: &[Token], close: usize) -> bool {
    let mut depth = 0usize;
    let Some(open) = (0..=close).rev().find(|&k| {
        if tokens[k].is_punct(")") {
            depth += 1;
        } else if tokens[k].is_punct("(") {
            depth -= 1;
        }
        depth == 0
    }) else {
        return false;
    };
    open > 0
        && (tokens[open - 1].is_ident_ci("if") || tokens[open - 1].is_ident_ci("elseif"))
        && tokens[open..close].iter().any(|t| VERSION_CHECKS.iter().any(|c| t.text.trim_start_matches('\\').eq_ignore_ascii_case(c)))
}

fn included_name(tokens: &[Token], k: usize) -> Option<String> {
    let file = tokens[k + 1..].iter().take_while(|t| !t.is_punct(";")).filter(|t| matches!(t.kind, TokenKind::Str { .. })).last()?;
    let name = file.text.trim_matches(|c| c == '\'' || c == '"');
    Some(name.rsplit(['/', '\\']).next().unwrap_or(name).to_ascii_lowercase())
}

const INCLUDES: [&str; 4] = ["require", "require_once", "include", "include_once"];

fn dead_includes(tokens: &[Token], dead: &[crate::guards::Span]) -> Vec<String> {
    (0..tokens.len())
        .filter(|&k| INCLUDES.iter().any(|w| tokens[k].is_ident_ci(w)))
        .filter(|&k| {
            dead.iter().any(|&(start, end)| (tokens[k].line, tokens[k].column) >= start && (tokens[k].line, tokens[k].column) <= end)
        })
        .filter_map(|k| included_name(tokens, k))
        .collect()
}

fn version_includes(tokens: &[Token]) -> Vec<String> {
    let mut out = Vec::new();
    for (k, t) in tokens.iter().enumerate() {
        if !["require", "require_once", "include", "include_once"].iter().any(|w| t.is_ident_ci(w)) {
            continue;
        }
        let mut guarded = k > 0 && tokens[k - 1].is_punct(")") && version_condition(tokens, k - 1);
        let mut depth = 0usize;
        for j in (0..k).rev() {
            if guarded {
                break;
            }
            if tokens[j].is_punct("}") {
                depth += 1;
            } else if tokens[j].is_punct("{") {
                if depth == 0 {
                    guarded = j > 0 && tokens[j - 1].is_punct(")") && version_condition(tokens, j - 1);
                } else {
                    depth -= 1;
                }
            }
        }
        if !guarded {
            continue;
        }
        let file = tokens[k + 1..].iter().take_while(|t| !t.is_punct(";")).filter(|t| matches!(t.kind, TokenKind::Str { .. })).last();
        if let Some(file) = file {
            let name = file.text.trim_matches(|c| c == '\'' || c == '"');
            out.push(name.rsplit(['/', '\\']).next().unwrap_or(name).to_ascii_lowercase());
        }
    }
    out
}
