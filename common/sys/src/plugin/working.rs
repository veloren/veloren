 
        fn raw_retreive_action(env: &EmitActionEnv, ptr: u32, len: u32) -> (u32, i32) {
            let memory: &Memory = if let Some(e) = env.memory.get_ref() {
                e
            } else {
                // This should not be possible but I prefer be safer!
                tracing::error!("Can't get memory from: `{}` plugin", env.name);
                return ();
            };
            let memory: MemoryView<u8> = memory.view();

            let str_slice = &memory[ptr as usize..(ptr + len) as usize];

            let bytes: Vec<u8> = str_slice.iter().map(|x| x.get()).collect();

            let r = env.ecs.load(std::sync::atomic::Ordering::SeqCst);
            if r == i32::MAX {
                println!("No ECS availible 1");
                return;
            }
            unsafe {
                if let Some(t) = (r as *const World).as_ref() {
                    println!("We have a pointer there");
                } else {
                    println!("No ECS availible 2");
                }
            }
        }