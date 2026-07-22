// common/Utils.ets

export class Utils {
  // 生成 UUID v7 (时间排序)
  static generateUuidV7(): string {
    const timestamp = Date.now();
    const random = Math.random().toString(16).slice(2) + Math.random().toString(16).slice(2);
    return `${timestamp.toString(16).padStart(12, '0')}-${random.slice(0, 4)}-${random.slice(4, 8)}-${random.slice(8, 12)}-${random.slice(12, 24)}`;
  }
  
  // 获取当前时间戳（毫秒）
  static getCurrentTimestamp(): number {
    return Date.now();
  }
  
  // 格式化时间
  static formatTime(timestamp: number, format: string): string {
    const date = new Date(timestamp);
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    const seconds = String(date.getSeconds()).padStart(2, '0');
    
    return format
      .replace('YYYY', String(year))
      .replace('MM', month)
      .replace('DD', day)
      .replace('HH', hours)
      .replace('mm', minutes)
      .replace('ss', seconds);
  }
  
  // 相对时间显示
  static getRelativeTime(timestamp: number): string {
    const now = Date.now();
    const diff = now - timestamp;
    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);
    
    if (days > 7) {
      return Utils.formatTime(timestamp, 'YYYY-MM-DD');
    } else if (days > 0) {
      return `${days}天前`;
    } else if (hours > 0) {
      return `${hours}小时前`;
    } else if (minutes > 0) {
      return `${minutes}分钟前`;
    } else {
      return '刚刚';
    }
  }
  
  // 内容预览
  static getContentPreview(content: string, maxLength: number = 100): string {
    if (content.length <= maxLength) {
      return content;
    }
    return content.substring(0, maxLength) + '...';
  }
  
  // 清理搜索文本
  static sanitizeSearchText(text: string): string {
    return text.toLowerCase().trim().replace(/[^\w\u4e00-\u9fa5]/g, ' ');
  }
  
  // 解析标签
  static parseTags(tagsString: string): string[] {
    return tagsString.split(/[,，\s]+/).filter((tag: string) => tag.length > 0);
  }
  
  // 标签去重
  static dedupTags(tags: string[]): string[] {
    const tagSet: Set<string> = new Set();
    for (let i = 0; i < tags.length; i++) {
      tagSet.add(tags[i]);
    }
    const result: string[] = [];
    tagSet.forEach((value: string) => {
      result.push(value);
    });
    return result;
  }
  
  // 延迟执行
  static delay(ms: number): Promise<void> {
    return new Promise((resolve: () => void) => setTimeout(resolve, ms));
  }
  
  // 防抖
  static debounce(func: Function, wait: number): Function {
    let timeout: number | null = null;
    return (...args: Object[]): void => {
      if (timeout !== null) {
        clearTimeout(timeout);
      }
      timeout = setTimeout((): void => {
        func(args);
      }, wait);
    };
  }
  
  /**
   * 将错误转换为 Error 对象
   */
  static toError(error: Object): Error {
    if (error instanceof Error) {
      return error;
    } else {
      return new Error(String(error));
    }
  }

  /**
   * 安全地获取错误消息
   */
  static getErrorMessage(error: Error): string {
    return error.message;
  }
}