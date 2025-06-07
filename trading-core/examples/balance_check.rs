use trading_core::blockchain::{BlockchainManager, error::BlockchainError}; // 引入 trading_core 区块链模块中的 BlockchainManager 和 BlockchainError

#[tokio::main] // Tokio 异步运行时宏，用于执行异步 main 函数
async fn main() -> Result<(), BlockchainError> { // 异步主函数，返回 BlockchainError 或空元组
    let blockchain = BlockchainManager::new("ws://127.0.0.1:9944").await?; // 创建一个新的 BlockchainManager 实例，连接到本地 Substrate 节点
    
    let address = blockchain.get_test_account(); // 获取测试账户地址
    println!("Test account address: {}", address); // 打印测试账户地址
    
    let balance = blockchain.get_account_balance(&address).await?; // 获取账户余额
    println!("Balance:"); // 打印 "Balance:"
    println!("  Free: {} planck", balance.free); // 打印可用余额 (单位: planck)
    println!("  Reserved: {} planck", balance.reserved); // 打印保留余额 (单位: planck)
    println!("  Total: {} planck", balance.total); // 打印总余额 (单位: planck)
    
    Ok(()) // 返回成功
}

// use trading_core::blockchain::{BlockchainManager, error::BlockchainError}; // 引入 trading_core 区块链模块中的 BlockchainManager 和 BlockchainError
// use sp_keyring::AccountKeyring; // 引入 sp_keyring 库中的 AccountKeyring，用于管理账户密钥对

// #[tokio::main] // Tokio 异步运行时宏
// async fn main() -> Result<(), BlockchainError> { // 异步主函数
//     // 1. 连接到本地节点 // 1. 连接到本地节点
//     println!("Connecting to local node..."); // 打印连接信息
//     let blockchain = BlockchainManager::new("ws://127.0.0.1:9944").await?; // 创建 BlockchainManager 实例并连接
    
//     // 2. 准备账户 // 2. 准备账户
//     let alice = AccountKeyring::Alice.pair();  // 获取 Alice 账户的密钥对
//     let bob_address = AccountKeyring::Bob.to_account_id().to_string(); // 获取 Bob 账户的地址并转换为字符串
//     println!("Bob's address: {}", bob_address); // 打印 Bob 的地址

//     // 3. 查询转账前的历史 // 3. 查询转账前的历史
//     println!("\nBefore transfer - Checking Alice's transfer history..."); // 打印查询 Alice 转账历史的信息
//     let alice_address = AccountKeyring::Alice.to_account_id().to_string(); // 获取 Alice 账户的地址
//     let history_before = blockchain.get_transfer_history(&alice_address).await?; // 获取 Alice 转账前的历史记录
//     println!("Found {} historical events", history_before.len()); // 打印找到的历史事件数量
//     for event in history_before { // 遍历历史事件
//         println!("Block #{} - {} event:", event.block_number, event.event_type); // 打印区块号和事件类型
//         println!("  Parameters: {:?}", event.params); // 打印事件参数
//     }

//     // 4. 执行转账 // 4. 执行转账
//     let transfer_amount = 100_000_000_000_000;  // 定义转账金额 (单位: planck)
//     println!("\nTransferring {} planck from Alice to Bob...", transfer_amount); // 打印转账信息
    
//     let result = blockchain.transfer(alice, &bob_address, transfer_amount).await?;  // 执行从 Alice 到 Bob 的转账操作
//     println!("Transfer successful!"); // 打印转账成功信息
//     println!("Transaction details:"); // 打印交易详情
//     println!("  From: {}", result.from); // 打印转出方地址
//     println!("  To: {}", result.to); // 打印转入方地址
//     println!("  Amount: {}", result.amount); // 打印转账金额
//     println!("  Block hash: {}", result.block_hash); // 打印区块哈希
//     println!("  Block number: {}", result.block_number); // 打印区块号

//     // 5. 等待几秒确保交易完成 // 5. 等待几秒确保交易完成
//     tokio::time::sleep(tokio::time::Duration::from_secs(6)).await; // 异步等待 6 秒

//     // 6. 查询转账后的历史 // 6. 查询转账后的历史
//     println!("\nAfter transfer - Checking Alice's transfer history..."); // 打印查询 Alice 转账历史的信息
//     let history_after = blockchain.get_transfer_history(&alice_address).await?; // 获取 Alice 转账后的历史记录
//     println!("Found {} historical events", history_after.len()); // 打印找到的历史事件数量
//     for event in history_after { // 遍历历史事件
//         println!("Block #{} - {} event:", event.block_number, event.event_type); // 打印区块号和事件类型
//         println!("  Parameters: {:?}", event.params); // 打印事件参数
//     }

//     Ok(()) // 返回成功
// }
