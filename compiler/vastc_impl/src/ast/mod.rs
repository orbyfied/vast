use vastc_supplemental::source::SourceSpan;

/// A managing structure for the whole AST, contains the allocation
/// arena for the nodes.
pub struct Ast {
  arena: Vec<AstNode>,
  root: Option<NodeRef>,
}

impl Ast {
  pub fn empty() -> Self {
    let mut arena: Vec<AstNode> = Vec::new();

    // reserve index zero for the invalid/uninitialized node
    arena.push(AstNode {
      kind: AstKind::Invalid,
      span: 0..0,
      index: UNINIT_NODE_REF
    });

    Self {
      arena,
      root: None
    }
  }
  
  pub fn resolved_root(&mut self, root: NodeRef) -> NodeRef {
    self.root = Some(root);
    root
  }

  pub fn get(&self, ptr: NodeRef) -> &AstNode {
    &self.arena.get(ptr.0).expect("invalid node ref? node refs must only be constructed by the Ast managing object")
  }

  pub fn get_mut(&mut self, ptr: NodeRef) -> &mut AstNode {
    self.arena.get_mut(ptr.0).expect("invalid node ref? node refs must only be constructed by the Ast managing object")
  }

  pub fn insert(&mut self, mut node: AstNode) -> NodeRef {
    let index = self.arena.len();
    node.index = NodeRef(index);
    self.arena.push(node);
    NodeRef(index)
  }

  pub fn make(&mut self, kind: AstKind, span: SourceSpan) -> NodeRef {
    let index = self.arena.len();
    self.arena.push(AstNode {
      kind,
      span,
      index: NodeRef(index)
    });
    
    NodeRef(index)
  }
}

/// Represents the data of a node in an abstract syntax tree.
#[derive(Clone, Debug)]
pub struct AstNode {
  index: NodeRef, // self-aware index, useful
  kind: AstKind,
  span: SourceSpan, // the span this node covers
}

impl AstNode {
  pub fn partial(kind: AstKind, span: SourceSpan) -> Self {
    Self {
      kind,
      span,
      index: UNINIT_NODE_REF
    }
  }

  pub fn insert(self, ast: &mut Ast) -> NodeRef {
    ast.insert(self)
  }

  pub fn reference(&self) -> NodeRef {
    assert!(self.index.is_valid());
    self.index
  }
}

/// Marker value, nodes must never be used without an initialized reference
/// unless for temporary passes of data or data initialization logic.
const UNINIT_NODE_REF: NodeRef = NodeRef(0);

/// Nodes are recursively contained through the mechanism of indices into
/// an arena of all the nodes flattened.
#[derive(Copy, Clone, PartialOrd, PartialEq, Debug)]
pub struct NodeRef(usize);

impl NodeRef {
  pub fn deref(self, ast: &Ast) -> &AstNode {
    ast.get(self)
  }

  pub fn deref_mut(self, ast: &mut Ast) -> &mut AstNode {
    ast.get_mut(self)
  }

  pub fn is_valid(&self) -> bool {
    self.0 != 0
  }
}

/// Represents the type and fields of a node in an abstract syntax tree.
/// These are the main node classes and their field definitions.
///
/// The `unit` lifetime parameter refers to the lifetime of the compilation unit
/// of one source file.
#[repr(u32)]
#[derive(Clone, Debug)]
pub enum AstKind {
  Invalid, // Marker value, reserved at index zero, must never escape index zero of the arena

  /// Represents an invalid sequence of tokens which was not immediately fatal to
  /// parsing. Contains the index into the parser' diagnostics vector.
  Error(usize /* index into `Parser.diagnostics` */),


}