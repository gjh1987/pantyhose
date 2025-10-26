/// Task trait - 任务接口
/// 所有异步任务都应该实现这个trait
pub trait Task: Send {
    /// 获取任务ID
    fn task_id(&self) -> u64;

    /// 设置任务ID
    fn set_task_id(&mut self, id: u64);
    
    /// 检查任务是否完成
    fn is_done(&self) -> bool;
    
    /// 任务完成后的处理
    /// 这个方法会在主线程中调用，可以安全地访问游戏状态
    fn done(&mut self);
}