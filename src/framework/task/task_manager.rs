use super::task::Task;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::{debug, info, warn, error};
use tokio::sync::Notify;

/// 任务管理器
/// 管理所有异步任务，在主线程中检查和处理完成的任务
/// 单例，被Server持有，支持跨线程访问
pub struct TaskManager {
    /// 下一个任务ID - 线程安全自增
    next_task_id: Arc<AtomicU64>,
    /// 任务列表 (task_id -> Task) - 线程安全
    task_list: Arc<Mutex<HashMap<u64, Box<dyn Task>>>>,
    /// 完成任务列表 (task_id -> Task) - 线程安全
    finish_task_list: Arc<Mutex<HashMap<u64, Box<dyn Task>>>>,
    /// Server的notify引用，用于唤醒主循环
    notify: Option<Arc<Notify>>,
}

impl TaskManager {
    /// 创建新的任务管理器
    pub fn new() -> Self {
        Self {
            next_task_id: Arc::new(AtomicU64::new(1)),
            task_list: Arc::new(Mutex::new(HashMap::new())),
            finish_task_list: Arc::new(Mutex::new(HashMap::new())),
            notify: None,
        }
    }
    
    /// 初始化任务管理器
    pub fn init(&mut self, notify: Arc<Notify>) -> bool {
        
        self.next_task_id.store(1, Ordering::SeqCst);
        self.task_list.lock().unwrap().clear();
        self.finish_task_list.lock().unwrap().clear();
        self.notify = Some(notify);
        
        true
    }
    
    /// 生成唯一的任务ID
    /// 自动递增并检查是否被占用，确保返回未使用的ID
    pub fn generate_task_id(&self) -> u64 {
        loop {
            // 获取下一个ID
            let task_id = self.next_task_id.fetch_add(1, Ordering::SeqCst);
            
            // 检查是否被占用（需要同时检查两个列表）
            let task_list = self.task_list.lock().unwrap();
            let finish_list = self.finish_task_list.lock().unwrap();
            
            if !task_list.contains_key(&task_id) && !finish_list.contains_key(&task_id) {
                // 找到未被占用的ID
                debug!("Generated unique task id: {}", task_id);
                return task_id;
            }
            
            // ID被占用，继续循环寻找下一个
            debug!("Task id {} is already in use, trying next", task_id);
        }
    }
    
    /// 添加任务（线程安全）- 自动分配任务ID
    pub fn add_task(&self, mut task: Box<dyn Task>) {
        // 生成唯一的任务ID
        let task_id = self.generate_task_id();
        
        // 设置任务ID
        task.set_task_id(task_id);
        
        // 添加到任务列表
        let mut task_list = self.task_list.lock().unwrap();
        task_list.insert(task_id, task);
        debug!("Added task with auto-assigned id {}", task_id);
    }
    
    /// 完成任务 - 将任务从task_list移到finish_task_list并通知Server
    /// 这个方法可以从任何线程调用
    pub fn finish_task(&self, task_id: u64) {
        // 从task_list获取任务
        let task = {
            let mut task_list = self.task_list.lock().unwrap();
            task_list.remove(&task_id)
        };
        
        if let Some(task) = task {
            // 放入finish_task_list
            {
                let mut finish_list = self.finish_task_list.lock().unwrap();
                finish_list.insert(task_id, task);
            }
            debug!("Task {} moved to finish_task_list", task_id);
            
            // 调用server的notify
            if let Some(ref notify) = self.notify {
                notify.notify_one();
                debug!("Notified server for task {}", task_id);
            } else {
                warn!("No notify available to wake server");
            }
        } else {
            error!("Task with id {} not found in task_list", task_id);
        }
    }
    
    /// 处理完成的任务
    /// 这个方法在主线程中调用
    /// drain()会清空finish_task_list
    pub fn process_finished_tasks(&mut self) {
        
        // 取出所有完成的任务（drain会清空HashMap）
        let finished_tasks = {
            let mut finish_list = self.finish_task_list.lock().unwrap();
            let tasks: Vec<(u64, Box<dyn Task>)> = finish_list.drain().collect();
            tasks
        };
        
        // 如果没有完成的任务，直接返回
        if finished_tasks.is_empty() {
            return ;
        }
        
        // 处理每个完成的任务
        for (task_id, mut task) in finished_tasks {
            if task.is_done() {
                task.done();
            } else {
                // 如果任务实际上没完成，放回task_list
                warn!("Task {} in finish_task_list but not done, moving back to task_list", task_id);
                let mut task_list = self.task_list.lock().unwrap();
                task_list.insert(task_id, task);
            }
        }
    }
    
    /// 检查是否有正在执行的任务
    pub fn has_pending_tasks(&self) -> bool {
        !self.task_list.lock().unwrap().is_empty()
    }
    
    /// 检查任务是否存在
    pub fn has_task(&self, task_id: u64) -> bool {
        self.task_list.lock().unwrap().contains_key(&task_id)
    }
    
    /// 移除任务
    pub fn remove_task(&self, task_id: u64) -> bool {
        if self.task_list.lock().unwrap().remove(&task_id).is_some() {
            debug!("Removed task {} from task_list", task_id);
            return true;
        }
        if self.finish_task_list.lock().unwrap().remove(&task_id).is_some() {
            debug!("Removed task {} from finish_task_list", task_id);
            return true;
        }
        false
    }
    
    /// 获取任务数量
    pub fn get_task_count(&self) -> usize {
        self.task_list.lock().unwrap().len()
    }
    
    /// 获取完成任务数量
    pub fn get_finished_task_count(&self) -> usize {
        self.finish_task_list.lock().unwrap().len()
    }
    
    /// 获取总任务数量
    pub fn get_total_task_count(&self) -> usize {
        let task_count = self.task_list.lock().unwrap().len();
        let finished_count = self.finish_task_list.lock().unwrap().len();
        task_count + finished_count
    }
    
    /// 清空所有任务
    pub fn clear_all_tasks(&mut self) {
        let task_count = self.task_list.lock().unwrap().len();
        let finished_count = self.finish_task_list.lock().unwrap().len();
        
        self.task_list.lock().unwrap().clear();
        self.finish_task_list.lock().unwrap().clear();
        
        if task_count > 0 || finished_count > 0 {
            warn!("Cleared {} tasks and {} finished tasks", task_count, finished_count);
        }
    }
    
    /// 清理管理器
    pub fn dispose(&mut self) {
        debug!("TaskManager disposing");
        
        let task_count = self.task_list.lock().unwrap().len();
        let finished_count = self.finish_task_list.lock().unwrap().len();
        
        self.clear_all_tasks();
        self.notify = None;
        
        info!("TaskManager disposed, cleared {} tasks and {} finished tasks", 
              task_count, finished_count);
    }
}

// 让TaskManager可以安全地跨线程共享
unsafe impl Send for TaskManager {}
unsafe impl Sync for TaskManager {}