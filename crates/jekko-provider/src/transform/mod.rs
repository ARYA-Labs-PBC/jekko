//! Request transforms: canonical message/tool/option shape -> provider-specific
//! request shape.
//!
//! Ported from `packages/jekko/src/provider/transform*.ts`. Each submodule
//! mirrors one TypeScript file:
//!
//! - [`mod@shared`]    : `transform-shared.ts`
//! - [`mod@message`]   : `transform-message.ts` (+ `transform-message-cache.ts`, `transform-message-utils.ts`)
//! - [`mod@options`]   : `transform-options.ts`
//! - [`mod@schema`]    : `transform-schema.ts`
//! - [`mod@variants`]  : `transform-variants.ts` (+ core/logic helpers)

pub mod message;
pub mod options;
pub mod schema;
pub mod shared;
pub mod variants;

pub use message::{message, ModelMessage, Part};
pub use options::{max_output_tokens, options, provider_options, small_options, OptionsInput};
pub use schema::schema;
pub use shared::{
    mime_to_modality, sanitize_surrogates, sdk_key, MimeModalityResult, Modality, OUTPUT_TOKEN_MAX,
};
pub use variants::{sampling_params, temperature, top_k, top_p, variants, SamplingParams};
