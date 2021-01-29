use anyhow::{anyhow, Context, Result};
use gas_estimation::{
    EthGasStation, GasNowGasStation, GasPriceEstimating, GnosisSafeGasStation,
    PriorityGasPriceEstimating, Transport,
};
use serde::de::DeserializeOwned;
use structopt::clap::arg_enum;
use web3::Web3;

arg_enum! {
    #[derive(Debug)]
    pub enum GasEstimatorType {
        EthGasStation,
        GasNow,
        GnosisSafe,
        Web3,
    }
}

#[derive(Clone)]
struct Client(reqwest::Client);

#[async_trait::async_trait]
impl Transport for Client {
    async fn get_json<'a, T: DeserializeOwned>(&self, url: &'a str) -> Result<T> {
        self.0
            .get(url)
            .send()
            .await
            .context("failed to make request")?
            .error_for_status()
            .context("response status is not success")?
            .json()
            .await
            .context("failed to decode response")
    }
}

pub async fn create_priority_estimator(
    client: &reqwest::Client,
    web3: &Web3<web3::transports::Http>,
    estimator_types: &[GasEstimatorType],
) -> Result<impl GasPriceEstimating> {
    let client = Client(client.clone());
    let network_id = web3.net().version().await?;
    let mut estimators = Vec::<Box<dyn GasPriceEstimating>>::new();
    for estimator_type in estimator_types {
        match estimator_type {
            GasEstimatorType::EthGasStation => {
                if !is_mainnet(&network_id) {
                    return Err(anyhow!("EthGasStation only supports mainnet"));
                }
                estimators.push(Box::new(EthGasStation::new(client.clone())))
            }
            GasEstimatorType::GasNow => {
                if !is_mainnet(&network_id) {
                    return Err(anyhow!("GasNow only supports mainnet"));
                }
                estimators.push(Box::new(GasNowGasStation::new(client.clone())))
            }
            GasEstimatorType::GnosisSafe => estimators.push(Box::new(
                GnosisSafeGasStation::with_network_id(&network_id, client.clone())?,
            )),
            GasEstimatorType::Web3 => estimators.push(Box::new(web3.clone())),
        }
    }
    Ok(PriorityGasPriceEstimating::new(estimators))
}

fn is_mainnet(network_id: &str) -> bool {
    network_id == "1"
}