use crate::plugin::wasm_env::HostFunctionEnvironment;
use wasmer::{AsStoreMut, AsStoreRef, FunctionEnvMut, Memory32, Memory64, MemorySize, WasmPtr};
// there is no WASI defined for wasm64, yet, so always use 32bit pointers
type MemoryModel = wasmer::Memory32;

trait PtrConversion<T, M: MemorySize> {
    fn convert(self) -> WasmPtr<T, M>
    where
        Self: Sized;
}

impl<T> PtrConversion<T, Memory64> for WasmPtr<T, Memory32> {
    fn convert(self) -> WasmPtr<T, Memory64>
    where
        Self: Sized,
    {
        WasmPtr::new(self.offset().into())
    }
}

fn print_impl(
    env: &HostFunctionEnvironment,
    store: &wasmer::StoreRef<'_>,
    ptr: WasmPtr<u8, MemoryModel>,
    len: <MemoryModel as wasmer::MemorySize>::Offset,
) -> Result<(), wasmer_wasix_types::wasi::Errno> {
    env.read_bytes(store, ptr.convert(), len.into())
        .map_err(|error| {
            tracing::error!(
                "Logging message from plugin {} failed with {:?}!",
                env.name,
                error
            );
            wasmer_wasix_types::wasi::Errno::Memviolation
        })
        .and_then(|bytes| {
            std::str::from_utf8(bytes.as_slice())
                .map_err(|error| {
                    tracing::error!(
                        "Logging message from plugin {} failed with {}!",
                        env.name,
                        error
                    );
                    wasmer_wasix_types::wasi::Errno::Inval
                })
                .map(|msg| tracing::info!("[{}]: {}", env.name, msg))
        })
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct CioVec {
    buf: WasmPtr<u8, MemoryModel>,
    buf_len: <MemoryModel as wasmer::MemorySize>::Offset,
}

// CioVec has no padding bytes, thus no action is necessary
unsafe impl wasmer::ValueType for CioVec {
    fn zero_padding_bytes(&self, _bytes: &mut [std::mem::MaybeUninit<u8>]) {
        const _: () = assert!(
            core::mem::size_of::<CioVec>()
                == core::mem::size_of::<WasmPtr<u8, MemoryModel>>()
                    + core::mem::size_of::<<MemoryModel as wasmer::MemorySize>::Offset>()
        );
    }
}

// fd_write(fd: fd, iovs: ciovec_array) -> Result<size, errno>
pub(crate) fn wasi_fd_write(
    mut env: FunctionEnvMut<HostFunctionEnvironment>,
    fd: i32,
    iov_addr: WasmPtr<CioVec, MemoryModel>,
    iov_len: <MemoryModel as wasmer::MemorySize>::Offset,
    out_result: WasmPtr<<MemoryModel as wasmer::MemorySize>::Offset, MemoryModel>,
) -> i32 {
    use wasmer_wasix_types::wasi::Errno;
    if fd != 1 && fd != 2 {
        Errno::Badf as i32
    } else {
        let memory = env.data().memory().clone();
        let mut written: u32 = 0;
        for i in 0..iov_len {
            let store = env.as_store_ref();
            let Ok(cio) = iov_addr
                .add_offset(i)
                .and_then(|p| p.read(&memory.view(&store)))
            else { return Errno::Memviolation as i32; };
            if let Err(e) = print_impl(env.data(), &store, cio.buf, cio.buf_len) {
                return e as i32;
            }
            written += cio.buf_len;
        }
        let store = env.as_store_mut();
        let mem = memory.view(&store);
        out_result
            .write(&mem, written)
            .map_or(Errno::Memviolation as i32, |()| Errno::Success as i32)
    }
}

//  environ_get(environ: Pointer<Pointer<u8>>, environ_buf: Pointer<u8>) ->
// Result<(), errno>
pub(crate) fn wasi_env_get(
    _env: FunctionEnvMut<HostFunctionEnvironment>,
    _environ: WasmPtr<WasmPtr<u8, MemoryModel>, MemoryModel>,
    _environ_buf: WasmPtr<u8, MemoryModel>,
) -> i32 {
    wasmer_wasix_types::wasi::Errno::Success as i32
}

// environ_sizes_get() -> Result<(size, size), errno>
pub(crate) fn wasi_env_sizes_get(
    mut env: FunctionEnvMut<HostFunctionEnvironment>,
    numptr: WasmPtr<u32, MemoryModel>,
    bytesptr: WasmPtr<u32, MemoryModel>,
) -> i32 {
    use wasmer_wasix_types::wasi::Errno;
    let memory = env.data().memory().clone();
    let store = env.as_store_mut();
    let mem = memory.view(&store);
    numptr
        .write(&mem, 0)
        .and_then(|()| bytesptr.write(&mem, 0))
        .map(|()| Errno::Success)
        .unwrap_or(Errno::Memviolation) as i32
}

// proc_exit(rval: exitcode)
pub(crate) fn wasi_proc_exit(env: FunctionEnvMut<HostFunctionEnvironment>, _exitcode: i32) {
    tracing::warn!("Plugin {} called exit().", env.data().name)
}
