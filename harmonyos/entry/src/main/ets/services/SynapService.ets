// services/SynapService.ets
import { Note } from '../models/NoteModel';
import { DatabaseService } from '../services/DatabaseService';
import { NoteRepository } from '../repository/NoteRepository';
import { Utils } from '../common/Utils';

export class SynapService {
  private databaseService: DatabaseService;
  private noteRepository: NoteRepository | null = null;
  private isInitialized: boolean = false;

  constructor(databaseService: DatabaseService) {
    this.databaseService = databaseService;
  }

  // 初始化服务
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      return;
    }
    
    try {
      await this.databaseService.initialize();
      this.noteRepository = new NoteRepository(this.databaseService);
      this.isInitialized = true;
      console.info('SynapService initialized successfully');
    } catch (error) {
      console.error('SynapService initialization failed:', error);
      // 使用 toError 转换错误并抛出
      throw Utils.toError(error);
    }
  }

  // 关闭数据库
  async closeDatabase(): Promise<void> {
    if (this.databaseService) {
      try {
        await this.databaseService.close();
        this.isInitialized = false;
        console.info('SynapService closed');
      } catch (error) {
        console.error('SynapService close failed:', error);
        throw Utils.toError(error);
      }
    }
  }

  // 检查服务是否已初始化
  private checkInitialized(): void {
    if (!this.isInitialized || !this.noteRepository) {
      throw new Error('SynapService not initialized');
    }
  }

  // 获取最近笔记
  async getRecentNotes(limit: number = 50): Promise<Note[]> {
    this.checkInitialized();
    return this.noteRepository!.getRecentNotes(limit);
  }

  // 获取单个笔记
  async getNote(noteId: string): Promise<Note | null> {
    this.checkInitialized();
    return this.noteRepository!.getNote(noteId);
  }

  // 创建笔记
  async createNote(content: string, tags: string[] = []): Promise<Note> {
    this.checkInitialized();
    return this.noteRepository!.createNote(content, tags);
  }

  // 编辑笔记
  async editNote(noteId: string, content: string, tags: string[] = []): Promise<Note> {
    this.checkInitialized();
    return this.noteRepository!.editNote(noteId, content, tags);
  }

  // 删除笔记
  async deleteNote(noteId: string): Promise<void> {
    this.checkInitialized();
    return this.noteRepository!.deleteNote(noteId);
  }

  // 搜索笔记
  async search(query: string): Promise<Note[]> {
    this.checkInitialized();
    return this.noteRepository!.searchNotes(query);
  }

  // 获取所有标签
  async getAllTags(): Promise<string[]> {
    this.checkInitialized();
    return this.noteRepository!.getAllTags();
  }

  // 获取笔记的回复
  async getReplies(parentId: string): Promise<Note[]> {
    this.checkInitialized();
    return this.noteRepository!.getReplies(parentId);
  }

  // 回复笔记
  async replyNote(parentId: string, content: string, tags: string[] = []): Promise<Note> {
    this.checkInitialized();
    return this.noteRepository!.replyNote(parentId, content, tags);
  }

  // 恢复笔记
  async restoreNote(noteId: string): Promise<void> {
    this.checkInitialized();
    return this.noteRepository!.restoreNote(noteId);
  }

  // 获取初始化状态
  get initialized(): boolean {
    return this.isInitialized;
  }
}