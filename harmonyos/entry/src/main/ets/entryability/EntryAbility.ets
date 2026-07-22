// entryability/EntryAbility.ets
import { AbilityConstant, ConfigurationConstant, UIAbility, Want } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { window } from '@kit.ArkUI';
import { SynapService } from '../services/SynapService';
import { DatabaseService } from '../services/DatabaseService';

const DOMAIN = 0x0000;

export default class EntryAbility extends UIAbility {
  private synapService: SynapService | null = null;

  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    void launchParam;
    
    try {
      this.context.getApplicationContext().setColorMode(ConfigurationConstant.ColorMode.COLOR_MODE_NOT_SET);
      
      hilog.info(DOMAIN, 'testTag', 'Starting service initialization...');
      
      // 初始化数据库服务
      const databaseService = new DatabaseService(this.context);
      
      // 初始化 SynapService
      this.synapService = new SynapService(databaseService);
      
      // 存储到 AppStorage 以便全局访问
      AppStorage.setOrCreate('synapService', this.synapService);
      
      hilog.info(DOMAIN, 'testTag', 'SynapService created successfully');
      
      // 异步初始化数据库 - 不阻塞 UI 加载
      this.synapService.initialize()
        .then(() => {
          hilog.info(DOMAIN, 'testTag', 'Database initialized successfully');
        })
        .catch((error: Error) => {
          hilog.error(DOMAIN, 'testTag', 'Database initialization failed: %{public}s', error.message);
          // 即使数据库初始化失败，也允许应用继续运行
        });
      
    } catch (err) {
      const error = err as Error;
      hilog.error(DOMAIN, 'testTag', 'Failed to initialize services. Cause: %{public}s', 
        JSON.stringify(error.message || error));
    }
    
    hilog.info(DOMAIN, 'testTag', '%{public}s', 'Ability onCreate');
  }

  onDestroy(): void {
    if (this.synapService) {
      this.synapService.closeDatabase()
        .catch((error: Error) => {
          hilog.error(DOMAIN, 'testTag', 'Failed to close database: %{public}s', error.message);
        });
      this.synapService = null;
    }
    hilog.info(DOMAIN, 'testTag', '%{public}s', 'Ability onDestroy');
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    hilog.info(DOMAIN, 'testTag', '%{public}s', 'Ability onWindowStageCreate');

    // 设置窗口为全屏
    windowStage.getMainWindowSync().setWindowLayoutFullScreen(true)
      .catch((err: Error) => {
        hilog.error(DOMAIN, 'testTag', 'Failed to set fullscreen: %{public}s', err.message);
      });

    // 加载主页面
    windowStage.loadContent('pages/Index', (err: Error | null): void => {
      if (err) {
        hilog.error(DOMAIN, 'testTag', 'Failed to load the content. Cause: %{public}s', 
          JSON.stringify(err.message || err));
        return;
      }
      hilog.info(DOMAIN, 'testTag', 'Succeeded in loading the content.');
    });
  }

  onWindowStageDestroy(): void {
    hilog.info(DOMAIN, 'testTag', '%{public}s', 'Ability onWindowStageDestroy');
  }

  onForeground(): void {
    hilog.info(DOMAIN, 'testTag', '%{public}s', 'Ability onForeground');
  }

  onBackground(): void {
    hilog.info(DOMAIN, 'testTag', '%{public}s', 'Ability onBackground');
  }
}