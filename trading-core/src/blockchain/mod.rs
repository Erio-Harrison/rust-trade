pub mod types; // 公开 types 模块，其中定义了区块链相关的各种数据结构
pub mod error; // 公开 error 模块，其中定义了区块链操作可能产生的错误类型

use std::str::FromStr; // 引入标准库的 FromStr trait，用于从字符串转换类型
use error::BlockchainError; // 从当前 crate 的 error 模块引入 BlockchainError 枚举
use subxt::{OnlineClient, PolkadotConfig}; // 引入 subxt 库的 OnlineClient 和 PolkadotConfig，用于与 Substrate 节点交互
use subxt::utils::AccountId32; // 引入 subxt 库工具模块中的 AccountId32 类型，表示 32 字节的账户 ID
use sp_keyring::AccountKeyring; // 引入 sp_keyring 库的 AccountKeyring，用于管理预定义的账户密钥对
use codec::Decode; // 引入 codec 库的 Decode trait，用于从字节流解码数据

#[subxt::subxt(runtime_metadata_path = "metadata.scale")] // subxt 宏，用于根据元数据文件（metadata.scale）生成运行时 API
pub mod polkadot {} // 定义 polkadot 模块，其中包含生成的与 Polkadot 运行时交互的 API

pub struct BlockchainManager { // 定义 BlockchainManager 结构体，用于管理与区块链的交互
    client: OnlineClient<PolkadotConfig>, // Substrate 客户端实例，配置为 Polkadot 类型
}

#[derive(Debug, Decode)] // 派生 Debug 和 Decode trait
struct AccountInfo { // 定义 AccountInfo 结构体，用于解码从链上获取的账户信息
    data: AccountData, // 包含账户数据
}

#[derive(Debug, Decode)] // 派生 Debug 和 Decode trait
struct AccountData { // 定义 AccountData 结构体，表示账户的具体余额信息
    free: u128, // 可用余额
    reserved: u128, // 保留余额
}

impl BlockchainManager { // 实现 BlockchainManager 的方法
    pub async fn new(node_url: &str) -> Result<Self, BlockchainError> { // 定义异步构造函数 new，根据节点 URL 创建实例
        let client = OnlineClient::<PolkadotConfig>::from_url(node_url) // 从给定的 URL 创建 OnlineClient 实例
            .await // 异步等待连接完成
            .map_err(|e| BlockchainError::ConnectionError(e.to_string()))?; // 如果出错，将错误映射为 BlockchainError::ConnectionError
            
        Ok(Self { client }) // 返回包含客户端实例的 BlockchainManager
    }

    pub fn get_client(&self) -> &OnlineClient<PolkadotConfig> { // 定义 get_client 方法，返回对客户端的不可变引用
        &self.client // 返回客户端引用
    }

    pub fn get_test_account(&self) -> String { // 定义 get_test_account 方法，获取一个测试账户地址
        let account = AccountKeyring::Alice.to_account_id(); // 获取 Alice 账户的 AccountId
        account.to_string() // 将 AccountId 转换为字符串并返回
    }

    pub async fn get_account_balance(&self, address: &str) -> Result<types::AccountBalance, BlockchainError> { // 定义异步方法 get_account_balance，获取指定地址的账户余额
        let storage = self.client.storage(); // 获取客户端的存储 API 访问器
        
        let at_block = storage.at_latest().await // 获取最新区块的存储状态
            .map_err(|e| BlockchainError::StorageError(e.to_string()))?; // 如果出错，映射为 BlockchainError::StorageError
        
        let account_id = AccountId32::from_str(address) // 从字符串地址解析为 AccountId32
            .map_err(|_| BlockchainError::InvalidAddress)?; // 如果解析失败，返回 BlockchainError::InvalidAddress
        
        let maybe_account = at_block // 在最新区块状态下查询账户信息
            .fetch(&subxt::dynamic::storage("System", "Account", vec![account_id])) // 动态构造存储查询：查询 System pallet 的 Account storage item，参数为 account_id
            .await // 异步等待查询完成
            .map_err(|e| BlockchainError::StorageError(e.to_string()))?; // 如果出错，映射为 BlockchainError::StorageError
    
        match maybe_account { // 匹配查询结果
            Some(account_data) => { // 如果找到了账户数据
                let account_info = AccountInfo::decode(&mut account_data.encoded()) // 从原始编码数据解码为 AccountInfo 结构体
                    .map_err(|e| BlockchainError::DecodeError(e.to_string()))?; // 如果解码失败，映射为 BlockchainError::DecodeError
                
                Ok(types::AccountBalance { // 返回解码后的账户余额信息
                    free: account_info.data.free, // 可用余额
                    reserved: account_info.data.reserved, // 保留余额
                    total: account_info.data.free + account_info.data.reserved, // 总余额
                })
            }
            None => Err(BlockchainError::AccountNotFound), // 如果未找到账户数据，返回 BlockchainError::AccountNotFound
        }
    }
    
    // pub async fn transfer( // 定义异步方法 transfer（已注释掉）
    //     &self, // 自身引用
    //     from_pair: Keypair, // 发送方密钥对
    //     to_address: &str, // 接收方地址字符串
    //     amount: u128 // 转账金额
    // ) -> Result<types::TransferDetails, BlockchainError> { // 返回转账详情或错误
    //     println!("Step 1: Converting addresses..."); // 打印步骤信息：转换地址
        
    //     // 转换目标地址 // 转换目标地址
    //     let to_account = AccountId32::from_str(to_address) // 从字符串解析接收方地址
    //         .map_err(|_| BlockchainError::InvalidAddress)?; // 如果失败，返回无效地址错误
    //     let dest = MultiAddress::Id(to_account); // 将 AccountId32 转换为 MultiAddress 类型
    
    //     println!("Step 2: Preparing transaction..."); // 打印步骤信息：准备交易
    //     let transfer_tx = polkadot::tx() // 构建交易
    //         .balances() // 选择 balances pallet
    //         .transfer_allow_death(dest, amount); // 调用 transfer_allow_death 方法，允许账户因余额不足而被销毁
    
    //     println!("Step 3: Submitting transaction..."); // 打印步骤信息：提交交易
        
    //     // 使用 from_pair 的原始公钥字节作为标识 // 使用 from_pair 的原始公钥字节作为标识
    //     let from_public = from_pair.public_key(); // 获取发送方公钥
    //     let from_address = format!("0x{}", hex::encode(from_public.as_ref())); // 将公钥转换为十六进制字符串地址
    
    //     let events = self.client // 客户端
    //         .tx() // 交易模块
    //         .sign_and_submit_then_watch( // 签名并提交交易，然后监听事件
    //             &transfer_tx, // 交易本身
    //             &from_pair, // 发送方密钥对用于签名
    //             Default::default() // 使用默认的交易参数
    //         )
    //         .await // 异步等待提交
    //         .map_err(|e| BlockchainError::TransactionError(format!("Failed to submit: {}", e)))? // 提交失败错误处理
    //         .wait_for_finalized_success() // 等待交易被最终区块确认并成功执行
    //         .await // 异步等待最终确认
    //         .map_err(|e| BlockchainError::TransactionError(format!("Failed to finalize: {}", e)))?; // 最终确认失败错误处理
    
    //     let transfer_event = events // 从交易事件中
    //         .find_first::<polkadot::balances::events::Transfer>() // 查找第一个 Balances::Transfer 事件
    //         .map_err(|e| BlockchainError::TransactionError(format!("Failed to find event: {}", e)))?; // 查找事件失败错误处理
    
    //     if let Some(event) = transfer_event { // 如果找到了 Transfer 事件
    //         println!("Transfer successful: {:?}", event); // 打印转账成功信息和事件详情
            
    //         let block = self.client // 客户端
    //             .blocks() // 区块模块
    //             .at_latest() // 获取最新区块
    //             .await // 异步等待
    //             .map_err(|e| BlockchainError::QueryError(e.to_string()))?; // 查询区块失败错误处理
    
    //         Ok(types::TransferDetails { // 返回转账详情
    //             from: from_address, // 发送方地址
    //             to: to_address.to_string(), // 接收方地址
    //             amount, // 转账金额
    //             block_hash: block.hash().to_string(), // 区块哈希
    //             block_number: block.number(), // 区块号
    //             success: true, // 交易成功标记
    //         })
    //     } else { // 如果未找到 Transfer 事件
    //         Err(BlockchainError::TransactionError("Transfer event not found".to_string())) // 返回交易事件未找到错误
    //     }
    // }

    pub async fn get_transfer_history(&self, address: &str) -> Result<Vec<types::BlockEvent>, BlockchainError> { // 定义异步方法 get_transfer_history，获取指定地址的转账历史
        let mut events = Vec::new(); // 初始化一个空的事件列表
        let account_id = AccountId32::from_str(address) // 从字符串地址解析为 AccountId32
            .map_err(|_| BlockchainError::InvalidAddress)?; // 如果解析失败，返回 BlockchainError::InvalidAddress
    
        let latest_block = self.client // 获取最新区块的信息
            .blocks() // 访问区块 API
            .at_latest() // 指定最新区块
            .await // 异步等待
            .map_err(|e| BlockchainError::QueryError(e.to_string()))?; // 如果出错，映射为 BlockchainError::QueryError
    
        let latest_number = latest_block.number(); // 获取最新区块号
        let start_block = latest_number.saturating_sub(100); // 计算起始区块号（最多查询最近 100 个区块，防止溢出）
    
        for number in (start_block..=latest_number).rev() { // 从起始区块号倒序遍历到最新区块号
            if let Ok(block) = self.client.blocks().at(latest_block.hash()).await { // 获取指定哈希的区块信息（注意：这里应该用 number 对应的哈希，而不是固定的 latest_block.hash()）
                if let Ok(events_result) = block.events().await { // 获取该区块的所有事件
                    for (event_idx, event) in events_result.iter().enumerate() { // 遍历每个事件及其索引
                        if let Ok(event) = event { // 如果事件解码成功
                            if event.pallet_name() == "Balances" && // 如果事件属于 Balances pallet
                               (event.variant_name() == "Transfer" || // 且事件类型是 Transfer
                                event.variant_name() == "Deposit" ||  // 或者 Deposit
                                event.variant_name() == "Withdraw") { // 或者 Withdraw
                                
                                let mut params = Vec::new(); // 初始化一个空的参数列表
                                while let Ok(field) = event.field_values() { // 迭代获取事件的字段值（注意：此方法可能不适用于所有事件结构，需要更健壮的解析）
                                    params.push(field.to_string()); // 将字段值转换为字符串并添加到参数列表
                                }
    
                                if params.iter().any(|p| p.contains(&account_id.to_string())) { // 如果参数中包含目标账户 ID
                                    events.push(types::BlockEvent { // 将该事件添加到历史事件列表中
                                        block_number: number, // 区块号
                                        block_hash: block.hash().to_string(), // 区块哈希
                                        event_index: event_idx as u32, // 事件索引
                                        event_type: event.variant_name().to_string(), // 事件类型
                                        params, // 事件参数
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    
        Ok(events) // 返回收集到的事件列表
    }
}