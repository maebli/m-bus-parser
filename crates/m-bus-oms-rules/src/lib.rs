#![no_std]

extern crate alloc;

#[macro_use]
mod sources;

pub mod address;
mod catalog;
mod commands;
pub mod layers;
pub mod lpwan;
pub mod readings;
pub mod sequence;
pub mod telegram;
pub mod tls_hello;
pub mod wired;

use m_bus_core::{Function, SecurityMode};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OmsSecurityModeCheck {
    pub result: &'static str,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub candidate_profile: Option<&'static str>,
}

impl OmsSecurityModeCheck {
    #[must_use]
    pub fn assess_5_0_1(function: Option<Function>, explicit_mode: Option<SecurityMode>) -> Self {
        let requires_zero = function.is_some_and(requires_mode_zero);
        let (result, candidate_profile) = match explicit_mode.map(|mode| mode.to_bits()) {
            None => ("undetermined", None),
            Some(n) if n != 0 && requires_zero => ("requires_mode_0", None),
            Some(0) => ("listed", None),
            Some(5) => ("listed", Some("A")),
            Some(7) => ("listed", Some("B")),
            Some(13) => ("listed", Some("C")),
            Some(10) => ("listed", Some("D")),
            Some(_) => ("not_listed", None),
        };
        Self {
            result,
            candidate_profile,
        }
    }
}

fn requires_mode_zero(function: Function) -> bool {
    matches!(
        function,
        Function::ReqUd1 { .. }
            | Function::ReqUd2 { .. }
            | Function::SndNk { .. }
            | Function::AccNr
            | Function::AccDmd
            | Function::Ack
            | Function::Nack
            | Function::CnfIr
    )
}
