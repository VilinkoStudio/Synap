// 笔记仓库 - 核心数据访问层

import relationalStore from '@ohos.data.relationalStore';
import { Note, NoteBrief, TimelineGroup, NoteFeedFilter, FilteredNoteStatus, TimelineDirection } from '../models/NoteModel';
import { Utils } from '../common/Utils';
import { Constants } from '../common/Constants';
import { DatabaseService } from '../services/DatabaseService';

export class NoteRepository {
  private dbService: DatabaseService;
  
  constructor(dbService: DatabaseService) {
    this.dbService = dbService;
  }
  
  // 创建笔记
  async createNote(content: string, tags: string[]): Promise<Note> {
    const store = this.dbService.getStore();
    const now = Utils.getCurrentTimestamp();
    const noteId = Utils.generateUuidV7();
    
    const values: relationalStore.ValuesBucket = {
      id: noteId,
      content: content,
      created_at: now,
      updated_at: now,
      deleted: 0
    };
    
    try {
      await store.insert(Constants.TABLE_NOTES, values);
      
      // 处理标签
      for (const tagName of tags) {
        await this.addTagToNote(noteId, tagName);
      }
      
      return await this.getNote(noteId) as Note;
    } catch (error) {
      console.error('创建笔记失败:', error);
      throw new Error(String(error));
    }
  }
  
  // 编辑笔记
  async editNote(noteId: string, newContent: string, newTags: string[]): Promise<Note> {
    const store = this.dbService.getStore();
    const now = Utils.getCurrentTimestamp();
    
    // 创建新版本
    const newNote = await this.createNote(newContent, newTags);
    
    // 建立编辑链接
    const linkValues: relationalStore.ValuesBucket = {
      previous_id: noteId,
      next_id: newNote.id,
      created_at: now
    };
    
    await store.insert(Constants.TABLE_EDIT_LINKS, linkValues);
    
    return newNote;
  }
  
  // 回复笔记
  async replyNote(parentId: string, content: string, tags: string[]): Promise<Note> {
    const store = this.dbService.getStore();
    const now = Utils.getCurrentTimestamp();
    
    // 创建子笔记
    const childNote = await this.createNote(content, tags);
    
    // 建立回复链接
    const linkValues: relationalStore.ValuesBucket = {
      parent_id: parentId,
      child_id: childNote.id,
      created_at: now
    };
    
    await store.insert(Constants.TABLE_REPLY_LINKS, linkValues);
    
    return childNote;
  }
  
  // 获取笔记
  async getNote(noteId: string): Promise<Note | null> {
    const store = this.dbService.getStore();
    
    const predicates = new relationalStore.RdbPredicates(Constants.TABLE_NOTES);
    predicates.equalTo('id', noteId);
    
    const resultSet = await store.query(predicates);
    
    try {
      if (resultSet.goToFirstRow()) {
        return await this.rowToNote(resultSet);
      }
      return null;
    } finally {
      resultSet.close();
    }
  }
  
  // 获取最近笔记
  async getRecentNotes(limit: number = Constants.DEFAULT_PAGE_SIZE): Promise<Note[]> {
    const store = this.dbService.getStore();
    
    const predicates = new relationalStore.RdbPredicates(Constants.TABLE_NOTES);
    predicates.equalTo('deleted', 0);
    predicates.orderByDesc('created_at');
    predicates.limitAs(limit);
    
    const resultSet = await store.query(predicates);
    
    try {
      const notes: Note[] = [];
      while (resultSet.goToNextRow()) {
        notes.push(await this.rowToNote(resultSet));
      }
      return notes;
    } finally {
      resultSet.close();
    }
  }
  
  // 搜索笔记
  async searchNotes(query: string, limit: number = Constants.SEARCH_LIMIT): Promise<Note[]> {
    const store = this.dbService.getStore();
    const searchTerms = Utils.sanitizeSearchText(query).split(' ').filter((t: string) => t.length > 0);
    
    const predicates = new relationalStore.RdbPredicates(Constants.TABLE_NOTES);
    predicates.equalTo('deleted', 0);
    
    // 简单的模糊搜索
    for (const term of searchTerms) {
      predicates.like('content', `%${term}%`);
    }
    
    predicates.orderByDesc('created_at');
    predicates.limitAs(limit);
    
    const resultSet = await store.query(predicates);
    
    try {
      const notes: Note[] = [];
      while (resultSet.goToNextRow()) {
        notes.push(await this.rowToNote(resultSet));
      }
      return notes;
    } finally {
      resultSet.close();
    }
  }
  
  // 删除笔记（软删除）
  async deleteNote(noteId: string): Promise<void> {
    const store = this.dbService.getStore();
    
    const values: relationalStore.ValuesBucket = {
      deleted: 1,
      updated_at: Utils.getCurrentTimestamp()
    };
    
    const predicates = new relationalStore.RdbPredicates(Constants.TABLE_NOTES);
    predicates.equalTo('id', noteId);
    
    await store.update(values, predicates);
  }
  
  // 恢复笔记
  async restoreNote(noteId: string): Promise<void> {
    const store = this.dbService.getStore();
    
    const values: relationalStore.ValuesBucket = {
      deleted: 0,
      updated_at: Utils.getCurrentTimestamp()
    };
    
    const predicates = new relationalStore.RdbPredicates(Constants.TABLE_NOTES);
    predicates.equalTo('id', noteId);
    
    await store.update(values, predicates);
  }
  
  // 获取笔记的回复
  async getReplies(parentId: string): Promise<Note[]> {
    const store = this.dbService.getStore();
    
    const sql = `
      SELECT n.* FROM ${Constants.TABLE_NOTES} n
      INNER JOIN ${Constants.TABLE_REPLY_LINKS} r ON n.id = r.child_id
      WHERE r.parent_id = ? AND n.deleted = 0
      ORDER BY n.created_at DESC
    `;
    
    const resultSet = await store.querySql(sql, [parentId]);
    
    try {
      const notes: Note[] = [];
      while (resultSet.goToNextRow()) {
        notes.push(await this.rowToNote(resultSet));
      }
      return notes;
    } finally {
      resultSet.close();
    }
  }
  
  // 获取所有标签
  async getAllTags(): Promise<string[]> {
    const store = this.dbService.getStore();
    
    const predicates = new relationalStore.RdbPredicates(Constants.TABLE_TAGS);
    predicates.orderByAsc('name');
    
    const resultSet = await store.query(predicates, ['name']);
    
    try {
      const tags: string[] = [];
      while (resultSet.goToNextRow()) {
        tags.push(resultSet.getString(resultSet.getColumnIndex('name')));
      }
      return tags;
    } finally {
      resultSet.close();
    }
  }
  
  // 添加标签到笔记
  private async addTagToNote(noteId: string, tagName: string): Promise<void> {
    const store = this.dbService.getStore();
    const now = Utils.getCurrentTimestamp();
    
    // 查找或创建标签
    let tagId = await this.findOrCreateTag(tagName, now);
    
    // 建立关联
    const values: relationalStore.ValuesBucket = {
      note_id: noteId,
      tag_id: tagId
    };
    
    await store.insert(Constants.TABLE_NOTE_TAGS, values);
  }
  
  // 查找或创建标签
  private async findOrCreateTag(tagName: string, timestamp: number): Promise<string> {
    const store = this.dbService.getStore();
    
    // 查找现有标签
    const predicates = new relationalStore.RdbPredicates(Constants.TABLE_TAGS);
    predicates.equalTo('name', tagName);
    
    const resultSet = await store.query(predicates, ['id']);
    
    try {
      if (resultSet.goToFirstRow()) {
        return resultSet.getString(resultSet.getColumnIndex('id'));
      }
    } finally {
      resultSet.close();
    }
    
    // 创建新标签
    const tagId = Utils.generateUuidV7();
    const values: relationalStore.ValuesBucket = {
      id: tagId,
      name: tagName,
      created_at: timestamp
    };
    
    await store.insert(Constants.TABLE_TAGS, values);
    return tagId;
  }
  
  // 获取笔记的标签
  private async getNoteTags(noteId: string): Promise<string[]> {
    const store = this.dbService.getStore();
    
    const sql = `
      SELECT t.name FROM ${Constants.TABLE_TAGS} t
      INNER JOIN ${Constants.TABLE_NOTE_TAGS} nt ON t.id = nt.tag_id
      WHERE nt.note_id = ?
      ORDER BY t.name
    `;
    
    const resultSet = await store.querySql(sql, [noteId]);
    
    try {
      const tags: string[] = [];
      while (resultSet.goToNextRow()) {
        tags.push(resultSet.getString(resultSet.getColumnIndex('name')));
      }
      return tags;
    } finally {
      resultSet.close();
    }
  }
  
  // 将数据库行转换为Note对象
  private async rowToNote(resultSet: relationalStore.ResultSet): Promise<Note> {
    const noteId = resultSet.getString(resultSet.getColumnIndex('id'));
    const tags = await this.getNoteTags(noteId);
    
    return {
      id: noteId,
      content: resultSet.getString(resultSet.getColumnIndex('content')),
      tags: tags,
      createdAt: resultSet.getLong(resultSet.getColumnIndex('created_at')),
      deleted: resultSet.getLong(resultSet.getColumnIndex('deleted')) === 1
    };
  }
}