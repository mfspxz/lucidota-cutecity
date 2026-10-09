//! cute-cms: the Cute City content-management core.
//!
//! - `model`: Post / Release / Project / District / SiteSettings
//! - `validate`: fail-closed content validation + the voice-pass gate
//! - `slug`: deterministic slug generation
//! - `schema`: D1 migrations (CMS core, admin_audit, page_views)
//! - `admin`: typed client over the secret-gated `/api/admin/*` routes

pub mod admin;
pub mod media;
pub mod model;
pub mod schema;
pub mod slug;
pub mod validate;
