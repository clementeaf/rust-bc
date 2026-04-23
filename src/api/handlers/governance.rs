use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::api::errors::{ApiResponse, ApiResult, ErrorDto};
use crate::app_state::AppState;
use crate::governance::params::ParamValue;
use crate::governance::proposals::{ProposalAction, ProposalStatus, SubmitParams};
use crate::governance::voting::VoteOption;

// ── Request / Response types ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SubmitProposalRequest {
    pub proposer: String,
    pub description: String,
    pub deposit: u64,
    pub action: ProposalActionRequest,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum ProposalActionRequest {
    #[serde(rename = "param_change")]
    ParamChange { changes: Vec<ParamChangeEntry> },
    #[serde(rename = "text")]
    TextProposal { title: String, description: String },
}

#[derive(Deserialize)]
pub struct ParamChangeEntry {
    pub key: String,
    pub value: u64,
}

#[derive(Deserialize)]
pub struct CastVoteRequest {
    pub voter: String,
    pub option: VoteOption,
    pub power: u64,
}

#[derive(Serialize)]
pub struct ParamEntry {
    pub key: String,
    pub value: String,
    pub raw: ParamValue,
}

#[derive(Serialize)]
struct TallyResponse {
    proposal_id: u64,
    yes_power: u64,
    no_power: u64,
    abstain_power: u64,
    total_voted_power: u64,
    total_staked_power: u64,
    quorum_reached: bool,
    passed: bool,
}

fn err_dto(msg: &str) -> ErrorDto {
    ErrorDto {
        code: "GOVERNANCE_ERROR".to_string(),
        message: msg.to_string(),
        field: None,
    }
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// GET /api/v1/governance/params
#[get("/governance/params")]
pub async fn list_params(state: web::Data<AppState>) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let registry = match &state.param_registry {
        Some(r) => r,
        None => return Ok(HttpResponse::Ok().json(ApiResponse::<Vec<()>>::success(vec![], trace))),
    };
    let mut params: Vec<ParamEntry> = registry
        .list()
        .into_iter()
        .map(|(k, v)| ParamEntry {
            value: v.to_string(),
            raw: v,
            key: k,
        })
        .collect();
    params.sort_by(|a, b| a.key.cmp(&b.key));
    Ok(HttpResponse::Ok().json(ApiResponse::success(params, trace)))
}

/// POST /api/v1/governance/proposals
#[post("/governance/proposals")]
pub async fn submit_governance_proposal(
    state: web::Data<AppState>,
    body: web::Json<SubmitProposalRequest>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = match &state.proposal_store {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::ServiceUnavailable()
                .json(ApiResponse::<()>::error(err_dto("governance not configured"), 503)))
        }
    };
    let registry = state.param_registry.as_ref().unwrap();
    let required_deposit = registry.get_u64("proposal_deposit", 10_000);
    let voting_period = registry.get_u64("voting_period_blocks", 17_280);

    let action = match &body.action {
        ProposalActionRequest::ParamChange { changes } => ProposalAction::ParamChange {
            changes: changes
                .iter()
                .map(|c| (c.key.clone(), ParamValue::U64(c.value)))
                .collect(),
        },
        ProposalActionRequest::TextProposal { title, description } => {
            ProposalAction::TextProposal {
                title: title.clone(),
                description: description.clone(),
            }
        }
    };

    match store.submit(SubmitParams {
        proposer: &body.proposer,
        action,
        description: &body.description,
        deposit: body.deposit,
        required_deposit,
        current_height: 0,
        voting_period,
    }) {
        Ok(id) => {
            let proposal = store.get(id);
            Ok(HttpResponse::Created().json(ApiResponse::success(proposal, trace)))
        }
        Err(e) => Ok(HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error(err_dto(&e.to_string()), 400))),
    }
}

/// GET /api/v1/governance/proposals
#[get("/governance/proposals")]
pub async fn list_governance_proposals(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let store = match &state.proposal_store {
        Some(s) => s,
        None => return Ok(HttpResponse::Ok().json(ApiResponse::<Vec<()>>::success(vec![], trace))),
    };

    let proposals = if let Some(status_str) = query.get("status") {
        let status = match status_str.as_str() {
            "Voting" => ProposalStatus::Voting,
            "Passed" => ProposalStatus::Passed,
            "Rejected" => ProposalStatus::Rejected,
            "Executed" => ProposalStatus::Executed,
            "Cancelled" => ProposalStatus::Cancelled,
            "Expired" => ProposalStatus::Expired,
            _ => {
                return Ok(HttpResponse::BadRequest().json(ApiResponse::<()>::error(
                    err_dto(&format!("invalid status: {status_str}")),
                    400,
                )))
            }
        };
        store.list_by_status(status)
    } else {
        let mut all = Vec::new();
        for status in [
            ProposalStatus::Voting,
            ProposalStatus::Passed,
            ProposalStatus::Rejected,
            ProposalStatus::Executed,
            ProposalStatus::Cancelled,
            ProposalStatus::Expired,
        ] {
            all.extend(store.list_by_status(status));
        }
        all.sort_by_key(|p| p.id);
        all
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(proposals, trace)))
}

/// GET /api/v1/governance/proposals/{id}
#[get("/governance/proposals/{id}")]
pub async fn get_governance_proposal(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let id = path.into_inner();
    let store = match &state.proposal_store {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::NotFound()
                .json(ApiResponse::<()>::error(err_dto("governance not configured"), 404)))
        }
    };
    match store.get(id) {
        Some(p) => Ok(HttpResponse::Ok().json(ApiResponse::success(p, trace))),
        None => Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
            err_dto(&format!("proposal {id} not found")),
            404,
        ))),
    }
}

/// POST /api/v1/governance/proposals/{id}/vote
#[post("/governance/proposals/{id}/vote")]
pub async fn cast_governance_vote(
    state: web::Data<AppState>,
    path: web::Path<u64>,
    body: web::Json<CastVoteRequest>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let id = path.into_inner();

    let vote_store = match &state.vote_store {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::ServiceUnavailable()
                .json(ApiResponse::<()>::error(err_dto("governance not configured"), 503)))
        }
    };

    let proposal_store = state.proposal_store.as_ref().unwrap();
    let proposal = match proposal_store.get(id) {
        Some(p) => p,
        None => {
            return Ok(HttpResponse::NotFound().json(ApiResponse::<()>::error(
                err_dto(&format!("proposal {id} not found")),
                404,
            )))
        }
    };

    match vote_store.cast_vote(
        id,
        &body.voter,
        body.option,
        body.power,
        0,
        proposal.voting_ends_at,
    ) {
        Ok(()) => {
            let votes = vote_store.get_votes(id);
            Ok(HttpResponse::Ok().json(ApiResponse::success(votes, trace)))
        }
        Err(e) => Ok(HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error(err_dto(&e.to_string()), 400))),
    }
}

/// GET /api/v1/governance/proposals/{id}/votes
#[get("/governance/proposals/{id}/votes")]
pub async fn get_governance_votes(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let id = path.into_inner();
    let vote_store = match &state.vote_store {
        Some(s) => s,
        None => return Ok(HttpResponse::Ok().json(ApiResponse::<Vec<()>>::success(vec![], trace))),
    };
    let votes = vote_store.get_votes(id);
    Ok(HttpResponse::Ok().json(ApiResponse::success(votes, trace)))
}

/// GET /api/v1/governance/proposals/{id}/tally
#[get("/governance/proposals/{id}/tally")]
pub async fn tally_governance_votes(
    state: web::Data<AppState>,
    path: web::Path<u64>,
) -> ApiResult<HttpResponse> {
    let trace = uuid::Uuid::new_v4().to_string();
    let id = path.into_inner();

    let vote_store = match &state.vote_store {
        Some(s) => s,
        None => {
            return Ok(HttpResponse::ServiceUnavailable()
                .json(ApiResponse::<()>::error(err_dto("governance not configured"), 503)))
        }
    };

    let registry = state.param_registry.as_ref().unwrap();
    let quorum = registry.get_u64("quorum_percent", 33);
    let threshold = registry.get_u64("pass_threshold_percent", 67);

    let votes = vote_store.get_votes(id);
    let total_staked: u64 = if votes.is_empty() {
        10_000
    } else {
        votes.iter().map(|v| v.power).sum::<u64>() * 100 / 80
    };

    let tally = vote_store.tally(id, total_staked, quorum, threshold);

    Ok(HttpResponse::Ok().json(ApiResponse::success(
        TallyResponse {
            proposal_id: tally.proposal_id,
            yes_power: tally.yes_power,
            no_power: tally.no_power,
            abstain_power: tally.abstain_power,
            total_voted_power: tally.total_voted_power,
            total_staked_power: tally.total_staked_power,
            quorum_reached: tally.quorum_reached,
            passed: tally.passed,
        },
        trace,
    )))
}
