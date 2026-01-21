//! Generated protobuf types and gRPC clients for MinKNOW API.
//!
//! This module includes the tonic-generated code from the MinKNOW protobuf definitions.

#![allow(clippy::all)]
#![allow(rustdoc::all)]
#![allow(dead_code)]

pub mod minknow_api {
    // Base types (no dependencies)
    pub mod read_end_reason {
        tonic::include_proto!("minknow_api.read_end_reason");
    }

    pub mod run_until {
        tonic::include_proto!("minknow_api.run_until");
    }

    pub mod analysis_workflows {
        tonic::include_proto!("minknow_api.analysis_workflows");
    }

    pub mod basecaller {
        tonic::include_proto!("minknow_api.basecaller");
    }

    // Types that depend on the above
    pub mod protocol {
        tonic::include_proto!("minknow_api.protocol");
    }

    pub mod acquisition {
        tonic::include_proto!("minknow_api.acquisition");
    }

    pub mod analysis_configuration {
        tonic::include_proto!("minknow_api.analysis_configuration");
    }

    pub mod device {
        tonic::include_proto!("minknow_api.device");
    }

    pub mod instance {
        tonic::include_proto!("minknow_api.instance");
    }

    pub mod manager {
        tonic::include_proto!("minknow_api.manager");
    }

    pub mod protocol_settings {
        tonic::include_proto!("minknow_api.protocol_settings");
    }

    pub mod statistics {
        tonic::include_proto!("minknow_api.statistics");
    }

    pub mod data {
        tonic::include_proto!("minknow_api.data");
    }
}
