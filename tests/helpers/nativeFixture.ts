import { expect, type Page } from '@playwright/test';

export async function nativeFixture(page: Page) {
  await page.addInitScript(()=>{
    const scope=window as unknown as Record<string,any>;
    const colors:Record<string,string>={red:'#d22638',blue:'#234ee0',green:'#2aa65c',gold:'#d7a02a',white:'#ffffff',mono:'#101010'};
    function artwork(color:string) {
      const canvas=document.createElement('canvas'); canvas.width=64; canvas.height=64;
      const context=canvas.getContext('2d')!;
      context.fillStyle=colors[color]; context.fillRect(0,0,64,64);
      return canvas.toDataURL('image/png');
    }
    let revision=1,id=0;
    const callbacks=new Map<number,(event:unknown)=>void>();
    const listeners=new Map<number,{event:string;handler:number}>();
    const desktop={alwaysOnTop:true,shortcut:'Ctrl+Alt+M',shortcutError:null,visible:true};
    let motion={revision:0,visible:true,animate:false,wasVisible:true};
    let reducedMotion=false;
    let startup=localStorage.getItem('fixture-startup')!=='false';
    let startupFailure=false;
    let snapshot={source:'spotify',status:'paused',title:'Soft focus',artist:'The Quiet Hours',album:'Test fixture',artworkDataUrl:artwork('red'),positionMs:86000,durationMs:222000,seekStartMs:0,seekEndMs:222000,lastTimelineUpdateUtcMs:null,snapshotAtUtcMs:Date.now(),playbackRate:1,canPlayPause:true,canPrevious:true,canNext:true,canSeek:true,volumeLevel:.75,volumeMuted:false,canSetVolume:true,errorCode:null,errorMessage:null,revision,sessionGeneration:1};
    const calls:string[]=[];
    const requests:{name:string;args:Record<string,unknown>}[]=[];
    function emit(event:string,payload:unknown) {for(const [eventId,entry] of listeners) if(entry.event===event) callbacks.get(entry.handler)?.({event,id:eventId,payload});}
    function publish(changes:Record<string,unknown>) { snapshot={...snapshot,...changes,revision:++revision,snapshotAtUtcMs:Date.now()}; emit('media://snapshot-updated',snapshot); }
    scope.isTauri=true;
    scope.__TAURI_EVENT_PLUGIN_INTERNALS__={unregisterListener(_event:string,eventId:number){listeners.delete(eventId);}};
    scope.__TAURI_INTERNALS__={metadata:{currentWindow:{label:'main'}},transformCallback(callback:(event:unknown)=>void){callbacks.set(++id,callback);return id;},async invoke(name:string,args:Record<string,any>={}){
      if(name==='plugin:event|listen'){listeners.set(++id,{event:args.event,handler:args.handler});return id;}
      if(name==='plugin:event|unlisten'){listeners.delete(args.eventId);return;}
      if(name==='get_desktop_status')return desktop;
      if(name==='get_startup_enabled')return startup;
      if(name==='get_media_snapshot'||name==='media_refresh')return snapshot;
      calls.push(name);
      requests.push({name,args});
      if(name==='set_startup_enabled'){
        if(startupFailure)throw new Error('Windows startup access denied');
        startup=!!args.enabled;localStorage.setItem('fixture-startup',String(startup));emit('desktop://startup',startup);return startup;
      }
      if(name==='window_motion_ready'){reducedMotion=!!args.reducedMotion;return motion;}
      if(name==='window_transition_complete'){
        if(args.revision===motion.revision){motion={...motion,animate:false};desktop.visible=motion.visible;emit('desktop://status',{...desktop});}
        return;
      }
      if(name==='media_play_pause')publish({status:snapshot.status==='playing'?'paused':'playing'});
      if(name==='hide_player'){desktop.visible=false;emit('desktop://status',{...desktop});return;}
      return {accepted:true,snapshot};
    }};
    scope.__ambientFixture={calls,requests,color(name:string){publish({artworkDataUrl:artwork(name)});},publish,
      failStartup(value:boolean){startupFailure=value;},
      event:emit,
      transition(visible:boolean){motion={revision:motion.revision+1,visible,animate:!reducedMotion,wasVisible:desktop.visible};if(visible||reducedMotion){desktop.visible=visible;emit('desktop://status',{...desktop});}emit('desktop://transition',{...motion});return motion.revision;},
      visible(visible:boolean){desktop.visible=visible;emit('desktop://status',{...desktop});},
      identity:()=>snapshot.artworkDataUrl,
      listeners:()=>[...listeners.values()].filter((entry)=>entry.event==='media://snapshot-updated').length};
  });
  await page.goto('/');
  await expect(page.locator('.ambient-backdrop')).not.toHaveAttribute('data-ambient-id','neutral');
}
