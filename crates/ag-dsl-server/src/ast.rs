#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    Literal(String),
    Param(String),
    Wildcard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub method: HttpMethod,
    pub path: Vec<PathSegment>,
    pub handler_capture: usize,
}

#[derive(Debug, Clone)]
pub struct ServerTemplate {
    pub name: String,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub middlewares: Vec<usize>,
    pub routes: Vec<Route>,
}
