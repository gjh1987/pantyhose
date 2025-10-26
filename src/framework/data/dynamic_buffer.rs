use std::cmp;
use std::fmt;
use bytes::BytesMut;

/// 动态缓冲区
/// 
/// 参考C#版本DynamicBuffer实现，提供动态扩容的字节缓冲区功能
/// 支持读写指针管理、大小端字节序读写、动态扩容等特性
#[derive(Clone)]
pub struct DynamicBuffer {
    /// 内部缓冲区
    buffer: BytesMut,
    /// 扩容大小
    expand_size: usize,
    /// 读指针位置
    read_index: usize,
    /// 写指针位置
    write_index: usize,
    /// 默认字节序（false为大端序，true为小端序）
    little_endian: bool,
}

impl DynamicBuffer {
    /// 创建新的动态缓冲区
    /// 
    /// # 参数
    /// * `init_size` - 初始大小，默认1024
    /// * `expand_size` - 扩容大小，默认1024
    pub fn new(init_size: usize, expand_size: usize) -> Self {
        Self {
            buffer: BytesMut::with_capacity(init_size),
            expand_size,
            read_index: 0,
            write_index: 0,
            little_endian: false,
        }
    }

    /// 使用默认参数创建缓冲区
    pub fn default() -> Self {
        Self::new(1024, 1024)
    }

    /// 设置默认字节序
    /// 
    /// # 参数
    /// * `little_endian` - true为小端序，false为大端序
    pub fn set_little_endian(&mut self, little_endian: bool) {
        self.little_endian = little_endian;
    }

    /// 获取当前默认字节序
    pub fn is_little_endian(&self) -> bool {
        self.little_endian
    }

    /// 获取缓冲区容量
    pub fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    /// 获取可丢弃的字节数（已读但未清理的数据）
    pub fn discardable_bytes(&self) -> usize {
        self.read_index
    }

    /// 获取可读字节数
    pub fn readable_bytes(&self) -> usize {
        self.write_index - self.read_index
    }

    /// 获取可写字节数
    pub fn writable_bytes(&self) -> usize {
        self.buffer.capacity() - self.write_index
    }

    /// 获取内部缓冲区引用
    pub fn byte_buffer(&self) -> &[u8] {
        &self.buffer[..]
    }

    /// 获取读指针位置
    pub fn read_index(&self) -> usize {
        self.read_index
    }

    /// 获取写指针位置
    pub fn write_index(&self) -> usize {
        self.write_index
    }

    /// 移动读指针
    /// 
    /// # 参数
    /// * `size` - 要移动的字节数
    pub fn read(&mut self, size: usize) {
        let actual_size = cmp::min(size, self.readable_bytes());
        self.read_index += actual_size;

        // 如果读写指针相等，清空缓冲区
        if self.read_index == self.write_index {
            self.clear();
        }
    }

    /// 移动写指针
    /// 
    /// # 参数
    /// * `size` - 要移动的字节数
    pub fn write(&mut self, size: usize) {
        let actual_size = cmp::min(size, self.writable_bytes());
        self.write_index += actual_size;
    }

    /// 预留可写字节空间
    /// 
    /// # 参数
    /// * `size` - 需要的可写字节数
    pub fn reserve_writable_bytes(&mut self, size: usize) {
        if self.writable_bytes() >= size {
            return;
        }

        let free_bytes = self.discardable_bytes() + self.writable_bytes();
        let current_capacity = self.buffer.capacity();
        let mut new_size = current_capacity;

        if free_bytes >= size && (free_bytes - size) < self.expand_size {
            new_size += self.expand_size;
        } else {
            let needed = size - free_bytes;
            let expand_count = (needed / self.expand_size) + 1;
            new_size += self.expand_size * expand_count;
        }

        let readable_bytes = self.readable_bytes();
        
        // 创建新的缓冲区
        let mut new_buffer = BytesMut::with_capacity(new_size);
        
        // 复制可读数据到新缓冲区开头
        if readable_bytes > 0 {
            new_buffer.extend_from_slice(&self.buffer[self.read_index..self.write_index]);
        }
        
        self.buffer = new_buffer;
        self.read_index = 0;
        self.write_index = readable_bytes;
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.read_index = 0;
        self.write_index = 0;
        self.buffer.clear();
    }

    /// 获取可读数据的切片
    pub fn readable_slice(&self) -> &[u8] {
        &self.buffer[self.read_index..self.write_index]
    }

    /// 获取可写区域的可变切片
    pub fn writable_slice_mut(&mut self) -> &mut [u8] {
        let len = self.buffer.len();
        if self.write_index >= len {
            // 需要扩容
            self.buffer.resize(self.write_index + 1, 0);
        }
        &mut self.buffer[self.write_index..]
    }

    // ===== Peek 操作 =====

    /// 查看8位整数（不移动读指针）
    pub fn peek_u8(&self, offset: usize) -> Option<u8> {
        if self.readable_bytes() < offset + 1 {
            return None;
        }
        Some(self.buffer[self.read_index + offset])
    }

    /// 查看16位整数（不移动读指针）
    pub fn peek_u16(&self, offset: usize) -> Option<u16> {
        if self.readable_bytes() < offset + 2 {
            return None;
        }
        let p = self.read_index + offset;
        let value = if self.little_endian {
            u16::from_le_bytes([self.buffer[p], self.buffer[p + 1]])
        } else {
            u16::from_be_bytes([self.buffer[p], self.buffer[p + 1]])
        };
        Some(value)
    }

    /// 查看32位整数（不移动读指针）
    pub fn peek_u32(&self, offset: usize) -> Option<u32> {
        if self.readable_bytes() < offset + 4 {
            return None;
        }
        let p = self.read_index + offset;
        let bytes = [
            self.buffer[p],
            self.buffer[p + 1],
            self.buffer[p + 2],
            self.buffer[p + 3],
        ];
        let value = if self.little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        };
        Some(value)
    }

    /// 查看64位整数（不移动读指针）
    pub fn peek_u64(&self, offset: usize) -> Option<u64> {
        if self.readable_bytes() < offset + 8 {
            return None;
        }
        let p = self.read_index + offset;
        let bytes = [
            self.buffer[p],
            self.buffer[p + 1],
            self.buffer[p + 2],
            self.buffer[p + 3],
            self.buffer[p + 4],
            self.buffer[p + 5],
            self.buffer[p + 6],
            self.buffer[p + 7],
        ];
        let value = if self.little_endian {
            u64::from_le_bytes(bytes)
        } else {
            u64::from_be_bytes(bytes)
        };
        Some(value)
    }

    // ===== Read 操作 =====

    /// 读取8位整数
    pub fn read_u8(&mut self) -> Option<u8> {
        let value = self.peek_u8(0)?;
        self.read(1);
        Some(value)
    }

    /// 读取16位整数
    pub fn read_u16(&mut self) -> Option<u16> {
        let value = self.peek_u16(0)?;
        self.read(2);
        Some(value)
    }

    /// 读取32位整数
    pub fn read_u32(&mut self) -> Option<u32> {
        let value = self.peek_u32(0)?;
        self.read(4);
        Some(value)
    }

    /// 读取64位整数
    pub fn read_u64(&mut self) -> Option<u64> {
        let value = self.peek_u64(0)?;
        self.read(8);
        Some(value)
    }

    // ===== Write 操作 =====

    /// 写入8位整数
    pub fn write_u8(&mut self, value: u8) {
        self.reserve_writable_bytes(1);
        if self.write_index >= self.buffer.len() {
            self.buffer.resize(self.write_index + 1, 0);
        }
        self.buffer[self.write_index] = value;
        self.write(1);
    }

    /// 写入16位整数
    pub fn write_u16(&mut self, value: u16) {
        self.reserve_writable_bytes(2);
        let bytes = if self.little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        
        let write_end = self.write_index + 2;
        if write_end > self.buffer.len() {
            self.buffer.resize(write_end, 0);
        }
        
        self.buffer[self.write_index..write_end].copy_from_slice(&bytes);
        self.write(2);
    }

    /// 写入32位整数
    pub fn write_u32(&mut self, value: u32) {
        self.reserve_writable_bytes(4);
        let bytes = if self.little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        
        let write_end = self.write_index + 4;
        if write_end > self.buffer.len() {
            self.buffer.resize(write_end, 0);
        }
        
        self.buffer[self.write_index..write_end].copy_from_slice(&bytes);
        self.write(4);
    }

    /// 写入64位整数
    pub fn write_u64(&mut self, value: u64) {
        self.reserve_writable_bytes(8);
        let bytes = if self.little_endian {
            value.to_le_bytes()
        } else {
            value.to_be_bytes()
        };
        
        let write_end = self.write_index + 8;
        if write_end > self.buffer.len() {
            self.buffer.resize(write_end, 0);
        }
        
        self.buffer[self.write_index..write_end].copy_from_slice(&bytes);
        self.write(8);
    }

    // ===== 字节数组操作 =====

    /// 读取字节数组
    /// 
    /// # 参数
    /// * `buffer` - 目标缓冲区
    /// * `offset` - 目标缓冲区偏移
    /// * `count` - 要读取的字节数
    /// 
    /// # 返回
    /// 实际读取的字节数
    pub fn read_bytes(&mut self, buffer: &mut [u8], offset: usize, count: usize) -> usize {
        let real_count = cmp::min(count, self.readable_bytes());
        if real_count == 0 || offset >= buffer.len() {
            return 0;
        }
        
        let copy_count = cmp::min(real_count, buffer.len() - offset);
        let src_start = self.read_index;
        let src_end = src_start + copy_count;
        let dst_start = offset;
        let dst_end = dst_start + copy_count;
        
        buffer[dst_start..dst_end].copy_from_slice(&self.buffer[src_start..src_end]);
        self.read_index += copy_count;

        if self.read_index == self.write_index {
            self.clear();
        }

        copy_count
    }

    /// 写入字节数组
    /// 
    /// # 参数
    /// * `buffer` - 源缓冲区
    /// * `offset` - 源缓冲区偏移
    /// * `count` - 要写入的字节数
    pub fn write_bytes(&mut self, buffer: &[u8], offset: usize, count: usize) {
        if offset >= buffer.len() || count == 0 {
            return;
        }
        
        let actual_count = cmp::min(count, buffer.len() - offset);
        self.reserve_writable_bytes(actual_count);
        
        let write_end = self.write_index + actual_count;
        if write_end > self.buffer.len() {
            self.buffer.resize(write_end, 0);
        }
        
        let src_start = offset;
        let src_end = src_start + actual_count;
        let dst_start = self.write_index;
        let dst_end = dst_start + actual_count;
        
        self.buffer[dst_start..dst_end].copy_from_slice(&buffer[src_start..src_end]);
        self.write_index += actual_count;
    }

    /// 写入整个字节切片
    pub fn write_slice(&mut self, data: &[u8]) {
        self.write_bytes(data, 0, data.len());
    }

    /// 读取所有可读数据到Vec
    pub fn read_all(&mut self) -> Vec<u8> {
        let readable = self.readable_bytes();
        if readable == 0 {
            return Vec::new();
        }
        
        let data = self.buffer[self.read_index..self.write_index].to_vec();
        self.clear();
        data
    }

    /// 获取数据的副本（不移动读指针）
    pub fn peek_all(&self) -> Vec<u8> {
        if self.readable_bytes() == 0 {
            return Vec::new();
        }
        self.buffer[self.read_index..self.write_index].to_vec()
    }

    /// 压缩缓冲区（移除已读数据）
    pub fn compact(&mut self) {
        if self.read_index == 0 {
            return;
        }
        
        let readable = self.readable_bytes();
        if readable == 0 {
            self.clear();
            return;
        }
        
        // 移动数据到缓冲区开头
        self.buffer.copy_within(self.read_index..self.write_index, 0);
        self.read_index = 0;
        self.write_index = readable;
    }

    /// 跳过指定字节数
    pub fn skip(&mut self, count: usize) {
        self.read(count);
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.readable_bytes() == 0
    }

    /// 获取统计信息
    pub fn stats(&self) -> BufferStats {
        BufferStats {
            capacity: self.capacity(),
            readable_bytes: self.readable_bytes(),
            writable_bytes: self.writable_bytes(),
            discardable_bytes: self.discardable_bytes(),
            read_index: self.read_index,
            write_index: self.write_index,
        }
    }
}

/// 缓冲区统计信息
#[derive(Debug, Clone)]
pub struct BufferStats {
    /// 容量
    pub capacity: usize,
    /// 可读字节数
    pub readable_bytes: usize,
    /// 可写字节数
    pub writable_bytes: usize,
    /// 可丢弃字节数
    pub discardable_bytes: usize,
    /// 读指针位置
    pub read_index: usize,
    /// 写指针位置
    pub write_index: usize,
}

impl fmt::Debug for DynamicBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynamicBuffer")
            .field("capacity", &self.capacity())
            .field("readable_bytes", &self.readable_bytes())
            .field("writable_bytes", &self.writable_bytes())
            .field("read_index", &self.read_index)
            .field("write_index", &self.write_index)
            .field("expand_size", &self.expand_size)
            .field("little_endian", &self.little_endian)
            .finish()
    }
}

impl Default for DynamicBuffer {
    fn default() -> Self {
        Self::new(1024, 1024)
    }
}

