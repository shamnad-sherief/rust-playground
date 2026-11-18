use std::net::SocketAddr;

use tonic::{Request, Response, Status, transport::Server};

use calculator::{AddRequest, AddResponse, DivideRequest, DivideResponse};

use crate::calculator::calculator_server::{Calculator, CalculatorServer};

pub mod calculator {
    tonic::include_proto!("calculator");
}

#[derive(Debug, Default)]
pub struct MyCalculator;

#[tonic::async_trait]
impl Calculator for MyCalculator {
    async fn add(&self, request: Request<AddRequest>) -> Result<Response<AddResponse>, Status> {
        let req = request.into_inner();
        println!("Server: received Add({}, {})", req.a, req.b);

        let reply = AddResponse {
            result: req.a + req.b,
        };

        Ok(Response::new(reply))
    }

    async fn divide(
        &self,
        request: Request<DivideRequest>,
    ) -> Result<Response<DivideResponse>, Status> {
        let req = request.into_inner();
        println!(
            "Server: received Divide({}, {})",
            req.numerator, req.denominator
        );

        if req.denominator == 0.0 {
            return Err(Status::invalid_argument("Cannot divide by zero"));
        }

        let reply = DivideResponse {
            result: req.numerator / req.denominator,
        };

        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 50051));
    let calculator = MyCalculator::default();
    println!("Calculator gRPC server listening on {}", addr);

    Server::builder()
        .add_service(CalculatorServer::new(calculator))
        .serve(addr)
        .await?;

    Ok(())
}
