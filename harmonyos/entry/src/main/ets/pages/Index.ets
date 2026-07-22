// 主页面 - 笔记列表（鸿蒙原生设计）

import router from '@ohos.router';
import promptAction from '@ohos.promptAction';
import { Note } from '../models/NoteModel';
import { Utils } from '../common/Utils';
import { HomeViewModel } from '../viewmodel/HomeViewModel';
import { SynapService } from '../services/SynapService';

@Entry
@Component
struct Index {
  @State notes: Note[] = [];
  @State isLoading: boolean = false;
  @State error: string | null = null;
  @State searchQuery: string = '';
  @State showEditor: boolean = false;
  @State editorContent: string = '';
  @State editorTags: string = '';
  @State isSearchMode: boolean = false;
  
  private viewModel: HomeViewModel | null = null;
  private scroller: Scroller = new Scroller();
  
  aboutToAppear(): void {
    let synapService: SynapService | undefined = AppStorage.get<SynapService>('synapService');
    if (synapService) {
      this.viewModel = new HomeViewModel(synapService);
      
      // 设置观察者
      this.viewModel.setOnUpdate((notes: Note[]): void => {
        this.notes = notes;
      });
      
      this.viewModel.setOnLoadingChange((loading: boolean): void => {
        this.isLoading = loading;
      });
      
      this.viewModel.setOnError((error: string | null): void => {
        this.error = error;
      });
      
      // 加载笔记
      this.viewModel.loadNotes();
    }
  }
  
  build() {
    Column() {
      // 顶部状态栏
      Row() {
        Text('Synap')
          .fontSize(28)
          .fontWeight(FontWeight.Bold)
          .fontColor('#1A1A1A')
        
        Blank()
        
        // 搜索按钮
        if (!this.isSearchMode) {
          Row() {
            SymbolGlyph($r('sys.symbol.magnifyingglass'))
              .fontSize(24)
              .fontColor(['#1A1A1A'])
          }
          .width(40)
          .height(40)
          .borderRadius(20)
          .justifyContent(FlexAlign.Center)
          .backgroundColor('#F5F5F5')
          .onClick((): void => {
            this.isSearchMode = true;
          })
          
          Row() {
            Text('☰')
              .fontSize(20)
              .fontColor('#1A1A1A')
          }
          .width(40)
          .height(40)
          .borderRadius(20)
          .margin({ left: 8 })
          .justifyContent(FlexAlign.Center)
          .backgroundColor('#F5F5F5')
          .onClick((): void => {
            router.pushUrl({
              url: 'pages/Settings'
            }).catch((err: Error) => {
              console.error('Navigate to Settings failed:', err.message);
            });
          })
        }
      }
      .width('100%')
      .padding({ left: 20, right: 20, top: 12, bottom: 12 })
      .backgroundColor('#FFFFFF')
      
      // 搜索模式
      if (this.isSearchMode) {
        Row() {
          SymbolGlyph($r('sys.symbol.chevron_left'))
            .fontSize(24)
            .fontColor(['#1A1A1A'])
            .onClick((): void => {
              this.isSearchMode = false;
              this.searchQuery = '';
              this.viewModel?.loadNotes();
            })
          
          Search({
            placeholder: '搜索笔记...',
            value: this.searchQuery
          })
          .layoutWeight(1)
          .height(44)
          .backgroundColor('#F5F5F5')
          .borderRadius(12)
          .padding({ left: 12, right: 12 })
          .onChange((value: string): void => {
            this.searchQuery = value;
            this.viewModel?.searchNotes(value);
          })
          .onSubmit((): void => {
            this.viewModel?.searchNotes(this.searchQuery);
          })
        }
        .width('100%')
        .padding({ left: 16, right: 20, bottom: 12 })
        .backgroundColor('#FFFFFF')
        .transition(TransitionEffect.OPACITY)
      }
      
      // 主内容区
      if (this.isLoading) {
        Column() {
          LoadingProgress()
            .width(48)
            .height(48)
            .color('#007DFF')
          
          Text('加载中...')
            .fontSize(14)
            .fontColor('#999999')
            .margin({ top: 12 })
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
        .backgroundColor('#FAFAFA')
      } else if (this.error) {
        Column() {
          SymbolGlyph($r('sys.symbol.exclamationmark_circle'))
            .fontSize(56)
            .fontColor(['#FF6B6B'])
          
          Text(this.error)
            .fontSize(16)
            .fontColor('#FF6B6B')
            .margin({ top: 16 })
            .textAlign(TextAlign.Center)
          
          Button('重试')
            .fontSize(16)
            .fontColor('#FFFFFF')
            .backgroundColor('#007DFF')
            .borderRadius(12)
            .padding({ left: 32, right: 32 })
            .height(48)
            .margin({ top: 24 })
            .onClick((): void => {
              this.viewModel?.loadNotes();
            })
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
        .backgroundColor('#FAFAFA')
      } else if (this.notes.length === 0) {
        Column() {
          SymbolGlyph($r('sys.symbol.doc_text'))
            .fontSize(64)
            .fontColor(['#DDDDDD'])
          
          Text('还没有笔记')
            .fontSize(20)
            .fontColor('#666666')
            .margin({ top: 20 })
            .fontWeight(FontWeight.Medium)
          
          Text('点击右下角的 + 按钮\n创建你的第一条笔记')
            .fontSize(14)
            .fontColor('#999999')
            .margin({ top: 8 })
            .textAlign(TextAlign.Center)
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
        .backgroundColor('#FAFAFA')
      } else {
        List({ scroller: this.scroller }) {
          ForEach(this.notes, (note: Note, index: number) => {
            ListItem() {
              NoteCard({ 
                note: note, 
                isFirst: index === 0,
                isLast: index === this.notes.length - 1
              })
            }
            .swipeAction({ 
              end: this.deleteButton(note.id)
            })
          }, (note: Note): string => note.id)
        }
        .width('100%')
        .layoutWeight(1)
        .backgroundColor('#FAFAFA')
        .padding({ left: 16, right: 16, top: 8, bottom: 8 })
        .edgeEffect(EdgeEffect.Spring)
      }
      
      // 新建浮动按钮（FAB）
      Row() {
        SymbolGlyph($r('sys.symbol.plus'))
          .fontSize(28)
          .fontColor(['#FFFFFF'])
      }
      .width(60)
      .height(60)
      .borderRadius(30)
      .backgroundColor('#007DFF')
      .justifyContent(FlexAlign.Center)
      .shadow({
        radius: 12,
        color: '#007DFF33',
        offsetX: 0,
        offsetY: 4
      })
      .position({ x: '90%', y: '88%' })
      .translate({ x: '-50%', y: '-50%' })
      .onClick((): void => {
        this.showEditor = true;
      })
      
      // 编辑器弹窗（Bottom Sheet 风格）
      if (this.showEditor) {
        this.EditorSheet()
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor('#FFFFFF')
  }
  
  @Builder deleteButton(noteId: string) {
    Column() {
      Row() {
        SymbolGlyph($r('sys.symbol.trash'))
          .fontSize(24)
          .fontColor(['#FFFFFF'])
      }
      .width('100%')
      .height('100%')
      .justifyContent(FlexAlign.Center)
      .backgroundColor('#FF6B6B')
      .borderRadius(12)
      .onClick((): void => {
        this.viewModel?.deleteNote(noteId);
        promptAction.showToast({
          message: '笔记已删除',
          duration: 2000
        });
      })
    }
    .width(80)
    .height('100%')
  }
  
  @Builder EditorSheet() {
    Column() {
      // 顶部拖动指示器
      Row()
        .width(40)
        .height(4)
        .borderRadius(2)
        .backgroundColor('#DDDDDD')
        .margin({ top: 8 })
      
      // 标题栏
      Row() {
        Text('新建笔记')
          .fontSize(22)
          .fontWeight(FontWeight.Bold)
          .fontColor('#1A1A1A')
        
        Blank()
        
        Text('取消')
          .fontSize(16)
          .fontColor('#007DFF')
          .fontWeight(FontWeight.Medium)
          .onClick((): void => {
            this.showEditor = false;
            this.editorContent = '';
            this.editorTags = '';
          })
      }
      .width('100%')
      .padding({ left: 20, right: 20, top: 12 })
      
      // 内容输入
      TextArea({
        placeholder: '输入笔记内容...',
        text: this.editorContent
      })
      .width('100%')
      .height(200)
      .backgroundColor('#F5F5F5')
      .borderRadius(12)
      .margin({ top: 16, bottom: 16 })
      .padding(16)
      .fontSize(16)
      .fontColor('#1A1A1A')
      .placeholderColor('#999999')
      .onChange((value: string): void => {
        this.editorContent = value;
      })
      
      // 标签输入
      Row() {
        Text('#')
          .fontSize(20)
          .fontColor('#666666')
          .margin({ right: 12 })
          .fontWeight(FontWeight.Medium)
        
        TextInput({
          placeholder: '添加标签（用逗号分隔）',
          text: this.editorTags
        })
        .layoutWeight(1)
        .height(44)
        .backgroundColor('#F5F5F5')
        .borderRadius(8)
        .padding({ left: 12, right: 12 })
        .fontSize(14)
        .fontColor('#1A1A1A')
        .placeholderColor('#999999')
        .onChange((value: string): void => {
          this.editorTags = value;
        })
      }
      .width('100%')
      
      // 保存按钮
      Button('保存笔记')
        .width('100%')
        .height(52)
        .backgroundColor(this.editorContent.trim().length > 0 ? '#007DFF' : '#CCCCCC')
        .fontSize(16)
        .fontColor('#FFFFFF')
        .fontWeight(FontWeight.Medium)
        .borderRadius(12)
        .margin({ top: 20 })
        .onClick(async (): Promise<void> => {
          if (this.editorContent.trim().length === 0) {
            promptAction.showToast({
              message: '请输入笔记内容',
              duration: 2000
            });
            return;
          }
          
          const tags = Utils.parseTags(this.editorTags);
          
          try {
            await this.viewModel?.createNote(this.editorContent, tags);
            this.showEditor = false;
            this.editorContent = '';
            this.editorTags = '';
            promptAction.showToast({
              message: '笔记创建成功',
              duration: 2000
            });
          } catch (error) {
            promptAction.showToast({
              message: '创建失败，请重试',
              duration: 2000
            });
            console.error('创建笔记失败:', error);
          }
        })
    }
    .width('100%')
    .backgroundColor('#FFFFFF')
    .borderRadius({ topLeft: 20, topRight: 20 })
    .padding({ left: 20, right: 20, bottom: 32 })
    .position({ x: '50%', y: '100%' })
    .translate({ x: '-50%', y: this.showEditor ? '-100%' : '0%' })
    .animation({
      duration: 300,
      curve: Curve.EaseInOut
    })
  }
}

// 笔记卡片组件（鸿蒙原生设计）- 带平滑动效
@Component
struct NoteCard {
  @Prop note: Note;
  @Prop isFirst: boolean = false;
  @Prop isLast: boolean = false;
  
  @State isPressed: boolean = false;
  @State offsetX: number = 0;
  @State offsetY: number = 0;
  
  private startX: number = 0;
  private startY: number = 0;
  
  build() {
    Column() {
      // 内容 - 靠左对齐
      Text(this.note.content)
        .fontSize(16)
        .fontColor('#1A1A1A')
        .maxLines(3)
        .textOverflow({ overflow: TextOverflow.Ellipsis })
        .lineHeight(24)
        .width('100%')
        .textAlign(TextAlign.Start)
      
      // 标签 - 靠左对齐
      if (this.note.tags.length > 0) {
        Row() {
          ForEach(this.note.tags.slice(0, 3), (tag: string) => {
            Text(`#${tag}`)
              .fontSize(12)
              .fontColor('#007DFF')
              .backgroundColor('#E8F5E9')
              .borderRadius(4)
              .padding({ left: 8, right: 8, top: 2, bottom: 2 })
              .margin({ right: 6 })
          }, (tag: string) => tag)
          
          if (this.note.tags.length > 3) {
            Text(`+${this.note.tags.length - 3}`)
              .fontSize(12)
              .fontColor('#999999')
          }
        }
        .width('100%')
        .margin({ top: 8 })
        .justifyContent(FlexAlign.Start)
      }
      
      // 底部信息 - 时间靠右对齐
      Row() {
        Blank()
        
        Text(Utils.getRelativeTime(this.note.createdAt))
          .fontSize(12)
          .fontColor('#999999')
        
        SymbolGlyph($r('sys.symbol.chevron_right'))
          .fontSize(14)
          .fontColor(['#CCCCCC'])
          .margin({ left: 4 })
      }
      .width('100%')
      .margin({ top: 12 })
    }
    .width('100%')
    .padding(16)
    .backgroundColor('#FFFFFF')
    .borderRadius(12)
    .margin({ 
      top: this.isFirst ? 0 : 8, 
      bottom: this.isLast ? 0 : 8 
    })
    .shadow({
      radius: 2,
      color: '#0000000A',
      offsetX: 0,
      offsetY: 1
    })
    // 平滑动效 - 点击时卡片平移
    .translate({ 
      x: this.offsetX, 
      y: this.offsetY 
    })
    .animation({
      duration: 200,
      curve: Curve.EaseOut
    })
    .onTouch((event: TouchEvent): void => {
      if (event.type === TouchType.Down) {
        // 按下时：卡片向上和向右轻微移动
        this.isPressed = true;
        this.offsetX = 4;
        this.offsetY = -4;
        // 保存触摸起始位置
        this.startX = event.touches[0].screenX;
        this.startY = event.touches[0].screenY;
      } else if (event.type === TouchType.Move) {
        // 移动时：根据手指位置计算偏移量
        const currentX = event.touches[0].screenX;
        const currentY = event.touches[0].screenY;
        const deltaX = currentX - this.startX;
        const deltaY = currentY - this.startY;
        
        // 限制最大偏移量，防止卡片移出屏幕
        const maxOffset = 20;
        this.offsetX = Math.min(Math.max(deltaX * 0.3, 0), maxOffset);
        this.offsetY = Math.min(Math.max(deltaY * 0.3, -maxOffset), 0);
      } else if (event.type === TouchType.Up) {
        // 抬起时：卡片回弹到原位
        this.isPressed = false;
        this.offsetX = 0;
        this.offsetY = 0;
      } else if (event.type === TouchType.Cancel) {
        // 取消时：卡片回弹到原位
        this.isPressed = false;
        this.offsetX = 0;
        this.offsetY = 0;
      }
    })
    .onClick((): void => {
      router.pushUrl({
        url: 'pages/NoteDetail',
        params: { noteId: this.note.id }
      });
    })
  }
}