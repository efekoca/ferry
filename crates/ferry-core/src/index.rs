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
}

pub(crate) fn method_sig(tokens: &[Token], function: usize, close_body_scan: usize) -> Option<(String, MethodSig)> {
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
    let sig = MethodSig {
        is_static: modifier("static"),
        is_private: modifier("private"),
        is_abstract: modifier("abstract"),
        params: crate::analyzer::param_shapes(tokens, open, close),
        returns: crate::analyzer::return_type(tokens, close),
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
    types: HashMap<String, Vec<TypeInfo>>,
    interfaces: HashMap<String, Vec<InterfaceInfo>>,
}

#[derive(Debug, Clone)]
struct InterfaceInfo {
    name: String,
    fqn: String,
    parent_fqns: Vec<String>,
    parents: Vec<String>,
    declares_methods: bool,
    signatures: HashMap<String, MethodSig>,
}

impl ProjectIndex {
    pub fn build(root: &Path, files: &[PathBuf]) -> Self {
        let mut index = Self { framework: Some(crate::project::detect_framework(root)), ..Self::default() };
        let mut seen = HashSet::new();
        for path in files.iter().cloned().chain(composer_files(root)) {
            if !seen.insert(std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone())) {
                continue;
            }
            if std::fs::metadata(&path).is_ok_and(|m| m.len() > crate::project::MAX_FILE_BYTES) {
                continue;
            }
            if let Ok(bytes) = std::fs::read(&path) {
                index.add(&lex(&String::from_utf8_lossy(&bytes)).tokens);
            }
        }
        index
    }

    pub fn from_tokens(tokens: &[Token]) -> Self {
        let mut index = Self::default();
        index.add(tokens);
        index
    }

    fn add(&mut self, tokens: &[Token]) {
        let methods = crate::analyzer::method_tokens(tokens);
        let spaces = namespaces(tokens);
        let names = Names::new(tokens);
        for (i, t) in tokens.iter().enumerate() {
            if !t.is_ident_ci("function") || methods.contains(&i) {
                continue;
            }
            let name = tokens[i + 1..].iter().find(|t| !t.is_punct("&"));
            if let Some(name) = name.filter(|n| n.kind == TokenKind::Ident) {
                self.functions.insert(qualify(namespace_at(&spaces, i), &name.text));
            }
        }
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
            let signatures: HashMap<String, MethodSig> =
                (open..body_end).filter(|&k| tokens[k].is_ident_ci("function")).filter_map(|k| method_sig(tokens, k, body_end)).collect();
            let declares_methods = !signatures.is_empty() || tokens[open..body_end].iter().any(|t| t.is_ident_ci("function"));
            self.interfaces.entry(short(&name.text)).or_default().push(InterfaceInfo {
                name: name.text.clone(),
                fqn: qualify(namespace_at(&spaces, i), &name.text),
                parent_fqns,
                parents,
                declares_methods,
                signatures,
            });
        }
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
        self.collect(info, 0)
    }

    fn collect(&self, info: &TypeInfo, depth: usize) -> Option<HashSet<String>> {
        if info.open || depth > 32 {
            return None;
        }
        let mut props = info.props.clone();
        for t in &info.traits {
            props.extend(self.collect(self.unique(t)?, depth + 1)?);
        }
        if let Some(parent) = &info.parent {
            if parent == "stdclass" {
                return None;
            }
            props.extend(self.collect(self.unique(parent)?, depth + 1)?);
        }
        Some(props)
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
        let Some(mut decl) = type_decl(tokens, i) else { continue };
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

pub(crate) fn class_decls(tokens: &[Token]) -> Vec<TypeDecl> {
    type_decls(tokens).into_iter().map(|d| d.decl).filter(|d| !d.info.is_trait).collect()
}

fn type_decl(tokens: &[Token], kw: usize) -> Option<TypeDecl> {
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
            if let Some((name, sig)) = method_sig(tokens, k, close) {
                info.signatures.insert(name, sig);
            }
        } else if t.kind == TokenKind::Variable {
            let promoted = depth == 2
                && (0..k)
                    .rev()
                    .take_while(|&j| !tokens[j].is_punct("(") && !tokens[j].is_punct(","))
                    .any(|j| modifiers.iter().any(|m| tokens[j].is_ident_ci(m)));
            if depth == 1 || promoted {
                info.props.insert(t.text.to_ascii_lowercase());
            }
        }
        k += 1;
    }
    info.open = open_class;
    Some(TypeDecl { open, close, info })
}

fn composer_files(root: &Path) -> Vec<PathBuf> {
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
