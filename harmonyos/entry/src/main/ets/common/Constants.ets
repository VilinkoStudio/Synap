// 常量定义

export class Constants {
  // 数据库相关
  static readonly DB_NAME = 'synap.db';
  static readonly DB_VERSION = 1;
  
  // 表名
  static readonly TABLE_NOTES = 'notes';
  static readonly TABLE_TAGS = 'tags';
  static readonly TABLE_NOTE_TAGS = 'note_tags';
  static readonly TABLE_EDIT_LINKS = 'edit_links';
  static readonly TABLE_REPLY_LINKS = 'reply_links';
  
  // 默认值
  static readonly DEFAULT_PAGE_SIZE = 20;
  static readonly MAX_PAGE_SIZE = 100;
  static readonly SEARCH_LIMIT = 50;
  static readonly TAG_SEARCH_LIMIT = 10;
  
  // 时间格式
  static readonly DATE_FORMAT = 'YYYY-MM-DD';
  static readonly TIME_FORMAT = 'HH:mm:ss';
  static readonly DATETIME_FORMAT = 'YYYY-MM-DD HH:mm:ss';
  
  // 存储键
  static readonly STORAGE_KEY_PREFIX = 'synap_';
  static readonly STORAGE_DRAFT_KEY = 'synap_draft';
  static readonly STORAGE_SETTINGS_KEY = 'synap_settings';
}