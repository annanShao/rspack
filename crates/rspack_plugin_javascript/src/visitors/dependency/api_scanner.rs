use rspack_core::{
  BuildInfo, ConstDependency, Dependency, DependencyLocation, DependencyTemplate, ResourceData,
  RuntimeGlobals, RuntimeRequirementsDependency, SpanExt,
};
use swc_core::{
  common::{Spanned, SyntaxContext},
  ecma::{
    ast::{
      AssignExpr, AssignOp, CallExpr, Callee, Expr, Ident, Lit, Pat, PatOrExpr, UnaryExpr,
      VarDeclarator,
    },
    visit::{noop_visit_type, Visit, VisitWith},
  },
};

use super::expr_matcher;
use crate::{
  dependency::{ModuleArgumentDependency, WebpackIsIncludedDependency},
  no_visit_ignored_stmt,
  parser_plugin::JavascriptParserPlugin,
  utils::eval::{self, BasicEvaluatedExpression},
};

pub const WEBPACK_HASH: &str = "__webpack_hash__";
pub const WEBPACK_PUBLIC_PATH: &str = "__webpack_public_path__";
pub const WEBPACK_MODULES: &str = "__webpack_modules__";
pub const WEBPACK_MODULE: &str = "__webpack_module__";
pub const WEBPACK_RESOURCE_QUERY: &str = "__resourceQuery";
pub const WEBPACK_CHUNK_LOAD: &str = "__webpack_chunk_load__";
pub const WEBPACK_BASE_URI: &str = "__webpack_base_uri__";
pub const NON_WEBPACK_REQUIRE: &str = "__non_webpack_require__";
pub const SYSTEM_CONTEXT: &str = "__system_context__";
pub const WEBPACK_SHARE_SCOPES: &str = "__webpack_share_scopes__";
pub const WEBPACK_INIT_SHARING: &str = "__webpack_init_sharing__";
pub const WEBPACK_NONCE: &str = "__webpack_nonce__";
pub const WEBPACK_CHUNK_NAME: &str = "__webpack_chunkname__";
pub const WEBPACK_RUNTIME_ID: &str = "__webpack_runtime_id__";
pub const WEBPACK_IS_INCLUDE: &str = "__webpack_is_included__";
pub const WEBPACK_REQUIRE: &str = "__webpack_require__";

pub fn get_typeof_evaluate_of_api(sym: &str) -> Option<&str> {
  match sym {
    WEBPACK_REQUIRE => Some("function"),
    WEBPACK_HASH => Some("string"),
    WEBPACK_PUBLIC_PATH => Some("string"),
    WEBPACK_MODULES => Some("object"),
    WEBPACK_MODULE => Some("object"),
    WEBPACK_RESOURCE_QUERY => Some("string"),
    WEBPACK_CHUNK_LOAD => Some("function"),
    WEBPACK_BASE_URI => Some("string"),
    NON_WEBPACK_REQUIRE => None,
    SYSTEM_CONTEXT => Some("object"),
    WEBPACK_SHARE_SCOPES => Some("object"),
    WEBPACK_INIT_SHARING => Some("function"),
    WEBPACK_NONCE => Some("string"),
    WEBPACK_CHUNK_NAME => Some("string"),
    WEBPACK_RUNTIME_ID => None,
    WEBPACK_IS_INCLUDE => Some("function"),
    _ => None,
  }
}

pub fn get_typeof_const_of_api(sym: &str) -> Option<&str> {
  match sym {
    WEBPACK_IS_INCLUDE => Some("function"),
    _ => None,
  }
}

pub struct ApiParserPlugin;

impl JavascriptParserPlugin for ApiParserPlugin {
  fn evaluate_typeof(
    &self,
    expression: &swc_core::ecma::ast::Ident,
    start: u32,
    end: u32,
    unresolved_mark: swc_core::common::SyntaxContext,
  ) -> Option<BasicEvaluatedExpression> {
    if expression.span.ctxt == unresolved_mark {
      get_typeof_evaluate_of_api(expression.sym.as_ref() as &str)
        .map(|res| eval::evaluate_to_string(res.to_string(), start, end))
    } else {
      None
    }
  }
}

pub struct ApiScanner<'a> {
  pub unresolved_ctxt: SyntaxContext,
  pub module: bool,
  pub build_info: &'a mut BuildInfo,
  pub enter_assign: bool,
  pub resource_data: &'a ResourceData,
  pub presentational_dependencies: &'a mut Vec<Box<dyn DependencyTemplate>>,
  pub dependencies: &'a mut Vec<Box<dyn Dependency>>,
  pub ignored: &'a mut Vec<DependencyLocation>,
}

impl<'a> ApiScanner<'a> {
  pub fn new(
    unresolved_ctxt: SyntaxContext,
    resource_data: &'a ResourceData,
    dependencies: &'a mut Vec<Box<dyn Dependency>>,
    presentational_dependencies: &'a mut Vec<Box<dyn DependencyTemplate>>,
    module: bool,
    build_info: &'a mut BuildInfo,
    ignored: &'a mut Vec<DependencyLocation>,
  ) -> Self {
    Self {
      unresolved_ctxt,
      module,
      build_info,
      enter_assign: false,
      resource_data,
      presentational_dependencies,
      dependencies,
      ignored,
    }
  }
}

impl Visit for ApiScanner<'_> {
  noop_visit_type!();
  no_visit_ignored_stmt!();

  fn visit_var_declarator(&mut self, var_declarator: &VarDeclarator) {
    match &var_declarator.name {
      Pat::Ident(ident) => {
        self.enter_assign = true;
        ident.visit_children_with(self);
        self.enter_assign = false;
      }
      _ => var_declarator.name.visit_children_with(self),
    }
    var_declarator.init.visit_children_with(self);
  }

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr) {
    if matches!(assign_expr.op, AssignOp::Assign) {
      match &assign_expr.left {
        PatOrExpr::Pat(box Pat::Ident(ident)) => {
          self.enter_assign = true;
          ident.visit_children_with(self);
          self.enter_assign = false;
        }
        _ => assign_expr.left.visit_children_with(self),
      }
    }
    assign_expr.right.visit_children_with(self);
  }

  fn visit_ident(&mut self, ident: &Ident) {
    if ident.span.ctxt != self.unresolved_ctxt {
      return;
    }
    match ident.sym.as_ref() as &str {
      WEBPACK_REQUIRE => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::REQUIRE.name().into(),
            Some(RuntimeGlobals::REQUIRE),
          )));
      }
      WEBPACK_HASH => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            format!("{}()", RuntimeGlobals::GET_FULL_HASH).into(),
            Some(RuntimeGlobals::GET_FULL_HASH),
          )));
      }
      WEBPACK_PUBLIC_PATH => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::PUBLIC_PATH.name().into(),
            Some(RuntimeGlobals::PUBLIC_PATH),
          )));
      }
      WEBPACK_MODULES => {
        if self.enter_assign {
          return;
        }
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::MODULE_FACTORIES.name().into(),
            Some(RuntimeGlobals::MODULE_FACTORIES),
          )));
      }
      WEBPACK_RESOURCE_QUERY => {
        let resource_query = self.resource_data.resource_query.as_deref().unwrap_or("");
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            serde_json::to_string(resource_query)
              .expect("should render module id")
              .into(),
            None,
          )));
      }
      WEBPACK_CHUNK_LOAD => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::ENSURE_CHUNK.name().into(),
            Some(RuntimeGlobals::ENSURE_CHUNK),
          )));
      }
      WEBPACK_MODULE => {
        self
          .presentational_dependencies
          .push(Box::new(ModuleArgumentDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            None,
          )));
      }
      WEBPACK_BASE_URI => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::BASE_URI.name().into(),
            Some(RuntimeGlobals::BASE_URI),
          )));
      }
      NON_WEBPACK_REQUIRE => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            if self.module {
              self.build_info.need_create_require = true;
              "__WEBPACK_EXTERNAL_createRequire(import.meta.url)".into()
            } else {
              "require".into()
            },
            None,
          )));
      }
      SYSTEM_CONTEXT => self
        .presentational_dependencies
        .push(Box::new(ConstDependency::new(
          ident.span.real_lo(),
          ident.span.real_hi(),
          RuntimeGlobals::SYSTEM_CONTEXT.name().into(),
          Some(RuntimeGlobals::SYSTEM_CONTEXT),
        ))),
      WEBPACK_SHARE_SCOPES => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::SHARE_SCOPE_MAP.name().into(),
            Some(RuntimeGlobals::SHARE_SCOPE_MAP),
          )))
      }
      WEBPACK_INIT_SHARING => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::INITIALIZE_SHARING.name().into(),
            Some(RuntimeGlobals::INITIALIZE_SHARING),
          )))
      }
      WEBPACK_NONCE => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::SCRIPT_NONCE.name().into(),
            Some(RuntimeGlobals::SCRIPT_NONCE),
          )));
      }
      WEBPACK_CHUNK_NAME => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::CHUNK_NAME.name().into(),
            Some(RuntimeGlobals::CHUNK_NAME),
          )));
      }
      WEBPACK_RUNTIME_ID => {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            ident.span.real_lo(),
            ident.span.real_hi(),
            RuntimeGlobals::RUNTIME_ID.name().into(),
            Some(RuntimeGlobals::RUNTIME_ID),
          )));
      }
      _ => {}
    }
  }

  fn visit_expr(&mut self, expr: &Expr) {
    let span = expr.span();
    if self
      .ignored
      .iter()
      .any(|r| r.start() <= span.real_lo() && span.real_hi() <= r.end())
    {
      return;
    }

    if expr_matcher::is_require_cache(expr) {
      self
        .presentational_dependencies
        .push(Box::new(ConstDependency::new(
          expr.span().real_lo(),
          expr.span().real_hi(),
          RuntimeGlobals::MODULE_CACHE.name().into(),
          Some(RuntimeGlobals::MODULE_CACHE),
        )));
    } else if expr_matcher::is_webpack_module_id(expr) {
      self
        .presentational_dependencies
        .push(Box::new(RuntimeRequirementsDependency::new(
          RuntimeGlobals::MODULE_ID,
        )));
      self
        .presentational_dependencies
        .push(Box::new(ModuleArgumentDependency::new(
          expr.span().real_lo(),
          expr.span().real_hi(),
          Some("id"),
        )));
      return;
    }
    expr.visit_children_with(self);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr) {
    if let Callee::Expr(box Expr::Ident(ident)) = &call_expr.callee
      && ident.sym == WEBPACK_IS_INCLUDE
    {
      if call_expr.args.len() != 1 {
        return;
      }
      if let Some(arg) = call_expr.args.first() {
        let request = if arg.spread.is_some() {
          None
        } else {
          arg
            .expr
            .as_lit()
            .and_then(|lit| match lit {
              Lit::Str(str) => Some(str),
              _ => None,
            })
            .map(|str| str.value.clone())
        };

        if let Some(request) = request {
          self
            .dependencies
            .push(Box::new(WebpackIsIncludedDependency::new(
              call_expr.span().real_lo(),
              call_expr.span().real_hi(),
              request,
            )));
        }
      }
    }
    call_expr.visit_children_with(self);
  }
  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr) {
    if let box Expr::Ident(ident) = &unary_expr.arg {
      if let Some(res) = get_typeof_const_of_api(ident.sym.as_ref() as &str) {
        self
          .presentational_dependencies
          .push(Box::new(ConstDependency::new(
            unary_expr.span().real_lo(),
            unary_expr.span().real_hi(),
            format!("'{res}'").into(),
            None,
          )));
      }
    }
    unary_expr.visit_children_with(self);
  }
}
