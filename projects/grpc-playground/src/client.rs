use tonic::Request;


pub mod calculator {
    tonic::include_proto!("calculator");
}

use calculator::calculator_client::CalculatorClient;
use calculator::{AddRequest, DivideRequest};


#[tokio::main]
async  fn main()  -> Result<(), Box<dyn std::error::Error>>{
    let mut client = CalculatorClient::connect("http://127.0.0.1:50051").await?;
    println!("Connected to gRPC server at http://127.0.0.1:50051");

    let response = client.add(AddRequest{
        a : 10,
        b: 20,
    }).await?;

    println!("Response: {}", response.into_inner().result);
    

    Ok(())
}   
