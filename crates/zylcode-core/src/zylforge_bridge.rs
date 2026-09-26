//! ZylForge bridge — the ONLY module that knows about ZylForge (contract v1).
//!
//! # Why this exists
//!
//! `docs/governance/ZYLFORGE_INTEGRATION_CONTRACT.md` defines the
//! cross-product boundary. This module is its enforcement point:
//!
//! - capability descriptors are recorded **verbatim** — ZylCode never
//!   upgrades a ZylForge state or trims its limitations;
//! - `.zyl` packages are consumed read-only (hash + store + attach);
//! - missing/unknown states fail closed to `UNVERIFIED`, never assumed good;
//! - every recorded event names its provenance as ZylForge-attested.
//!
//! Live ZylForge connectivity is UNVERIFIED; every output carries that
//! qualifier so no UI can display ZylForge state as locally proven.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Bridge contract version (matches the governance document).
pub const ZYLFORGE_CONTRACT_VERSION: u32 = 1;

/// Connectivity truth for the whole bridge.
pub const CONNECTIVITY: ConnectivityState = ConnectivityState::Unverified;

/// Explicit, honest connectivity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectivityState {
    /// No live ZylForge round-trip has been proven from this build.
    Unverified,
    /// A live ZylForge instance answered (set only by runtime commissioning).
    Verified,
    /// ZylForge was reachable before and a current attempt failed.
    Blocked,
}

/// ZylForge capability descriptor, carried verbatim per contract §2.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityDescriptor {
    /// Stable capability ID (`zylforge.<workbench>.<operation>`).
    pub capability_id: String,
    /// Always `zylforge`.
    pub product: String,
    /// ZylForge workbench name.
    pub workbench: String,
    /// Operation name.
    pub operation: String,
    /// ZylForge's own state for this capability, **verbatim**.
    pub state: String,
    /// ZylForge evidence reference, verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_ref: Option<String>,
    /// ZylForge build/version string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// ZylForge's recorded limitations, verbatim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limitations: Option<String>,
}

impl CapabilityDescriptor {
    /// Ingest a descriptor from a ZylForge payload. Fails closed: a missing
    /// state becomes `UNVERIFIED`; nothing is upgraded, nothing trimmed.
    pub fn from_payload(value: &serde_json::Value) -> Result<Self> {
        let mut desc: CapabilityDescriptor = serde_json::from_value(value.clone())
            .map_err(|e| {
                // Preserve as much as possible: an unparseable descriptor is
                // recorded UNVERIFIED rather than discarded when the id is
                // recoverable.
                anyhow::anyhow!("capability descriptor unparseable: {e}")
            })
            .or_else(|_| {
                let id = value
                    .get("capability_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("zylforge.unknown.unknown");
                Ok::<CapabilityDescriptor, anyhow::Error>(CapabilityDescriptor {
                    capability_id: id.to_string(),
                    product: "zylforge".to_string(),
                    workbench: "unknown".to_string(),
                    operation: "unknown".to_string(),
                    state: "UNVERIFIED".to_string(),
                    evidence_ref: None,
                    version: None,
                    limitations: Some(format!(
                        "descriptor failed to parse; original payload: {value}"
                    )),
                })
            })?;
        // Contract §2 rule 2/3: a missing or empty state fails closed.
        if desc.state.trim().is_empty() {
            desc.state = "UNVERIFIED".to_string();
        }
        // Never allow a descriptor to claim to be from another product.
        if desc.product != "zylforge" {
            anyhow::bail!(
                "capability descriptor claims product {:?}; bridge is zylforge-only",
                desc.product
            );
        }
        Ok(desc)
    }

    /// True when ZylForge itself attests runtime verification.
    pub fn runtime_verified(&self) -> bool {
        self.state.eq_ignore_ascii_case("RUNTIME_VERIFIED")
            || self.state.eq_ignore_ascii_case("RUNTIME VERIFIED")
    }
}

/// A recorded cross-product event (contract §5). ZylCode records these in
/// its own Proof Engine / Artifact Bus with provenance naming ZylForge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeEvent {
    /// Event kind per contract §5.
    pub kind: BridgeEventKind,
    /// Capability involved, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_id: Option<String>,
    /// Human/provenance note (who approved, what ZylForge reported, …).
    pub note: String,
    /// The honest state recorded for this event.
    pub state: String,
}

/// Kinds of cross-product events ZylCode records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BridgeEventKind {
    OperationProposed,
    UserApproval,
    KernelResultReported,
    VerificationReported,
    ArtifactHashPinned,
    RuntimeCapture,
    BlockedJourney,
    UnsupportedFeature,
}

/// Build the Proof-Engine note for a ZylForge-reported kernel result. The
/// note always attributes the claim to ZylForge — ZylCode never upgrades it.
pub fn kernel_result_note(descriptor: &CapabilityDescriptor, report: &str) -> String {
    format!(
        "zylforge-attested kernel result for {}: {report} (zylforge state: {}; connectivity: unverified)",
        descriptor.capability_id, descriptor.state
    )
}

/// The read-only `.zyl` consumption record (contract §3). All fields are
/// carried verbatim from the package's sidecar metadata; ZylCode adds only
/// its own hash and ids.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZylPackageRef {
    /// The package's own schema version, verbatim.
    pub schema_version: u32,
    pub project_id: String,
    /// Geometry provenance string from ZylForge.
    pub geometry_provenance: String,
    /// Feature-tree provenance string from ZylForge.
    pub feature_tree_provenance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telemetry_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_ref: Option<String>,
    /// ZylForge build identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_build: Option<String>,
}

impl ZylPackageRef {
    /// Parse a `.zyl` sidecar reference. ZylCode does not interpret the
    /// geometry payload — only this metadata envelope.
    pub fn parse(json: &str) -> Result<Self> {
        let r: ZylPackageRef = serde_json::from_str(json)
            .context("zyl package reference is not valid metadata JSON")?;
        anyhow::ensure!(
            r.schema_version > 0,
            "zyl package schema_version must be positive"
        );
        Ok(r)
    }

    /// The provenance string recorded alongside the artifact when this
    /// package enters the Artifact Bus (never rewritten afterwards).
    pub fn provenance_string(&self) -> String {
        format!(
            "zyl_package schema v{}; geometry: {}; feature tree: {}; build: {}; connectivity: unverified",
            self.schema_version,
            self.geometry_provenance,
            self.feature_tree_provenance,
            self.source_build.as_deref().unwrap_or("unknown")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_is_carried_verbatim() {
        let payload = serde_json::json!({
            "capability_id": "zylforge.modeling.fillet",
            "product": "zylforge",
            "workbench": "modeling",
            "operation": "fillet",
            "state": "PROPOSED",
            "evidence_ref": "zylforge://evidence/123",
            "version": "0.3.1",
            "limitations": "fillet radius > 10mm unverified"
        });
        let d = CapabilityDescriptor::from_payload(&payload).unwrap();
        // Verbatim: PROPOSED stays PROPOSED. ZylCode has no API to upgrade it.
        assert_eq!(d.state, "PROPOSED");
        assert!(!d.runtime_verified());
        assert_eq!(
            d.limitations.as_deref(),
            Some("fillet radius > 10mm unverified"),
            "limitations carried verbatim"
        );
    }

    #[test]
    fn missing_state_fails_closed_to_unverified() {
        let payload = serde_json::json!({
            "capability_id": "zylforge.mystery.op",
            "product": "zylforge",
            "workbench": "mystery",
            "operation": "op"
        });
        let d = CapabilityDescriptor::from_payload(&payload).unwrap();
        assert_eq!(d.state, "UNVERIFIED");
        assert!(!d.runtime_verified());
    }

    #[test]
    fn unparseable_descriptor_is_recorded_not_discarded() {
        let payload = serde_json::json!({ "capability_id": "zylforge.a.b", "product": "zylforge" });
        let d = CapabilityDescriptor::from_payload(&payload).unwrap();
        assert_eq!(d.state, "UNVERIFIED");
        assert!(d.limitations.unwrap().contains("failed to parse"));
    }

    #[test]
    fn foreign_product_is_refused() {
        let payload = serde_json::json!({
            "capability_id": "x.other.thing",
            "product": "zylcode",
            "workbench": "w",
            "operation": "o",
            "state": "RUNTIME_VERIFIED"
        });
        assert!(CapabilityDescriptor::from_payload(&payload).is_err());
    }

    #[test]
    fn runtime_verified_recognized_only_when_zylforge_says_so() {
        let payload = serde_json::json!({
            "capability_id": "zylforge.m.export",
            "product": "zylforge",
            "workbench": "m",
            "operation": "export",
            "state": "RUNTIME_VERIFIED"
        });
        let d = CapabilityDescriptor::from_payload(&payload).unwrap();
        assert!(d.runtime_verified());

        let payload2 = serde_json::json!({
            "capability_id": "zylforge.m.export",
            "product": "zylforge",
            "workbench": "m",
            "operation": "export",
            "state": "IMPLEMENTED"
        });
        let d2 = CapabilityDescriptor::from_payload(&payload2).unwrap();
        assert!(!d2.runtime_verified());
    }

    #[test]
    fn kernel_result_note_always_attributes_to_zylforge() {
        let d = CapabilityDescriptor {
            capability_id: "zylforge.modeling.fillet".into(),
            product: "zylforge".into(),
            workbench: "modeling".into(),
            operation: "fillet".into(),
            state: "PROPOSED".into(),
            evidence_ref: None,
            version: None,
            limitations: None,
        };
        let note = kernel_result_note(&d, "kernel returned success");
        assert!(note.contains("zylforge-attested"));
        assert!(note.contains("PROPOSED"), "state travels with the note");
        assert!(note.contains("connectivity: unverified"));
    }

    #[test]
    fn zyl_package_ref_roundtrip_and_validation() {
        let r = ZylPackageRef {
            schema_version: 2,
            project_id: "p1".into(),
            geometry_provenance: "occt fillet op 41".into(),
            feature_tree_provenance: "tree #7".into(),
            telemetry_ref: Some("tel://9".into()),
            verification_ref: Some("ver://3".into()),
            source_build: Some("zylforge 0.3.1".into()),
        };
        let json = serde_json::to_string(&r).unwrap();
        let back = ZylPackageRef::parse(&json).unwrap();
        assert_eq!(back.schema_version, 2);
        let prov = back.provenance_string();
        assert!(prov.contains("occt fillet op 41"));
        assert!(prov.contains("connectivity: unverified"));
        assert!(ZylPackageRef::parse("not json").is_err());
    }

    #[test]
    fn connectivity_is_honestly_unverified() {
        assert_eq!(CONNECTIVITY, ConnectivityState::Unverified);
    }
}
