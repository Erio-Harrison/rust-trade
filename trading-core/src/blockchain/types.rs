use serde::{Deserialize, Serialize}; // 引入 serde 库的 Deserialize 和 Serialize trait，用于序列化和反序列化

#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct AccountBalance { // 定义账户余额结构体
    pub free: u128, // 可用余额
    pub reserved: u128, // 保留余额
    pub total: u128, // 总余额
}

#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct TransferDetails { // 定义转账详情结构体
    pub from: String, // 转出方地址
    pub to: String, // 转入方地址
    pub amount: u128, // 转账金额
    pub block_hash: String, // 包含此交易的区块哈希
    pub block_number: u32, // 包含此交易的区块号
    pub success: bool, // 交易是否成功
}

#[derive(Debug, Clone, Serialize, Deserialize)] // 派生 Debug, Clone, Serialize, Deserialize trait
pub struct BlockEvent { // 定义区块事件结构体
    pub block_number: u32, // 区块号
    pub block_hash: String, // 区块哈希
    pub event_index: u32, // 事件在区块中的索引
    pub event_type: String, // 事件类型
    pub params: Vec<String>, // 事件参数列表（字符串形式）
}