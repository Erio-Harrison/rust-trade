#![allow(missing_docs)] // 允许缺少文档注释
use subxt::{OnlineClient, PolkadotConfig}; // 引入 subxt 库中的 OnlineClient 和 PolkadotConfig，用于与 Substrate 节点交互
use subxt_signer::sr25519::dev; // 引入 subxt_signer 库中的 sr25519::dev，用于获取预定义的开发账户

#[subxt::subxt(runtime_metadata_path = "metadata.scale")] // subxt 宏，用于根据元数据文件生成运行时 API
pub mod polkadot {} // 定义 polkadot 模块，其中包含生成的运行时 API

#[tokio::main] // Tokio 异步运行时宏
async fn main() -> Result<(), Box<dyn std::error::Error>> { // 异步主函数，返回 Result，错误类型为动态错误
    // Create a new API client, configured to talk to Polkadot nodes. // 创建一个新的 API 客户端，配置为与 Polkadot 节点通信。
    let api = OnlineClient::<PolkadotConfig>::new().await?; // 创建 OnlineClient 实例，连接到 Polkadot 网络

    // Build a balance transfer extrinsic. // 构建一个余额转账交易。
    let dest = dev::bob().public_key().into(); // 获取 Bob 账户的公钥，并转换为目标地址类型
    let balance_transfer_tx = polkadot::tx().balances().transfer_allow_death(dest, 10_000); // 构建一个余额转账交易，目标是 dest，金额是 10_000，允许账户被销毁

    // Submit the balance transfer extrinsic from Alice, and wait for it to be successful
    // and in a finalized block. We get back the extrinsic events if all is well.
    // 从 Alice 提交余额转账交易，并等待其成功并进入最终区块。
    // 如果一切顺利，我们将取回交易事件。
    let from = dev::alice(); // 获取 Alice 账户作为交易发送方
    let events = api // API 客户端
        .tx() // 获取交易模块
        .sign_and_submit_then_watch_default(&balance_transfer_tx, &from) // 签名并提交交易，然后监听默认事件
        .await? // 等待提交完成
        .wait_for_finalized_success() // 等待交易被最终区块确认并成功执行
        .await?; // 等待最终确认完成

    // Find a Transfer event and print it. // 查找 Transfer 事件并打印它。
    let transfer_event = events.find_first::<polkadot::balances::events::Transfer>()?; // 从交易事件中查找第一个 balances::Transfer 事件
    if let Some(event) = transfer_event { // 如果找到了 Transfer 事件
        println!("Balance transfer success: {event:?}"); // 打印转账成功信息和事件详情
    }

    Ok(()) // 返回成功
}