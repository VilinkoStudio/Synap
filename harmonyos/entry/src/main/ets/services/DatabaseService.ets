// 数据库服务 - 使用鸿蒙 RdbStore

import relationalStore from '@ohos.data.relationalStore';
import common from '@ohos.app.ability.common';
import { Constants } from '../common/Constants';

export class DatabaseService {
  private rdbStore: relationalStore.RdbStore | null = null;
  private context: common.Context;
  
  constructor(context: common.Context) {
    this.context = context;
  }
  
  // 初始化数据库
  async initialize(): Promise<void> {
    const config: relationalStore.StoreConfig = {
      name: Constants.DB_NAME,
      securityLevel: relationalStore.SecurityLevel.S1
    };
    
    try {
      this.rdbStore = await relationalStore.getRdbStore(this.context, config);
      await this.createTables();
    } catch (error) {
      console.error('数据库初始化失败:', error);
      // 始终创建新的 Error 对象
      throw new Error(String(error));
    }
  }
  
  // 创建表
  private async createTables(): Promise<void> {
    if (!this.rdbStore) {
      throw new Error('数据库未初始化');
    }
    
    // 创建笔记表
    const createNotesTable = `
      CREATE TABLE IF NOT EXISTS ${Constants.TABLE_NOTES} (
        id TEXT PRIMARY KEY,
        content TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        updated_at INTEGER NOT NULL,
        deleted INTEGER DEFAULT 0
      )
    `;
    
    // 创建标签表
    const createTagsTable = `
      CREATE TABLE IF NOT EXISTS ${Constants.TABLE_TAGS} (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL UNIQUE,
        created_at INTEGER NOT NULL
      )
    `;
    
    // 创建笔记-标签关联表
    const createNoteTagsTable = `
      CREATE TABLE IF NOT EXISTS ${Constants.TABLE_NOTE_TAGS} (
        note_id TEXT NOT NULL,
        tag_id TEXT NOT NULL,
        PRIMARY KEY (note_id, tag_id),
        FOREIGN KEY (note_id) REFERENCES ${Constants.TABLE_NOTES}(id) ON DELETE CASCADE,
        FOREIGN KEY (tag_id) REFERENCES ${Constants.TABLE_TAGS}(id) ON DELETE CASCADE
      )
    `;
    
    // 创建编辑链接表
    const createEditLinksTable = `
      CREATE TABLE IF NOT EXISTS ${Constants.TABLE_EDIT_LINKS} (
        previous_id TEXT NOT NULL,
        next_id TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (previous_id, next_id),
        FOREIGN KEY (previous_id) REFERENCES ${Constants.TABLE_NOTES}(id) ON DELETE CASCADE,
        FOREIGN KEY (next_id) REFERENCES ${Constants.TABLE_NOTES}(id) ON DELETE CASCADE
      )
    `;
    
    // 创建回复链接表
    const createReplyLinksTable = `
      CREATE TABLE IF NOT EXISTS ${Constants.TABLE_REPLY_LINKS} (
        parent_id TEXT NOT NULL,
        child_id TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        PRIMARY KEY (parent_id, child_id),
        FOREIGN KEY (parent_id) REFERENCES ${Constants.TABLE_NOTES}(id) ON DELETE CASCADE,
        FOREIGN KEY (child_id) REFERENCES ${Constants.TABLE_NOTES}(id) ON DELETE CASCADE
      )
    `;
    
    try {
      await this.rdbStore.executeSql(createNotesTable);
      await this.rdbStore.executeSql(createTagsTable);
      await this.rdbStore.executeSql(createNoteTagsTable);
      await this.rdbStore.executeSql(createEditLinksTable);
      await this.rdbStore.executeSql(createReplyLinksTable);
    } catch (error) {
      // 始终创建新的 Error 对象
      throw new Error('创建数据库表失败: ' + String(error));
    }
  }
  
  // 获取数据库实例
  getStore(): relationalStore.RdbStore {
    if (!this.rdbStore) {
      throw new Error('数据库未初始化');
    }
    return this.rdbStore;
  }
  
  // 关闭数据库
  async close(): Promise<void> {
    if (this.rdbStore) {
      try {
        await relationalStore.deleteRdbStore(this.context, Constants.DB_NAME);
        this.rdbStore = null;
      } catch (error) {
        // 始终创建新的 Error 对象
        throw new Error('关闭数据库失败: ' + String(error));
      }
    }
  }
}