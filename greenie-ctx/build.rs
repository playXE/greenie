macro_rules! arch_cc {
    ($os: literal, $arch: literal => $($file: expr),* ) => {
        #[cfg(all(target_arch = $arch, target_family = $os,not(feature="use-tsx")))]
        {
           cc::Build::new().$(file($file)).*.compile("libgreenie_ctx");

        }
        #[cfg(all(target_arch = $arch, target_family = $os,feature="use-tsx"))]
        {
            cc::Build::new().$(file($file)).*.define("USE_TSX",None).compile("libgreenie_ctx");
        }
    };
}

fn main() {
    arch_cc!("unix","i386" => "asm/jump_i386_sysv_elf_gas.S","asm/make_i386_sysv_elf_gas.S","asm/ontop_i386_sysv_elf_gas.S");
    arch_cc!("unix","x86_64" => "asm/jump_x86_64_sysv_elf_gas.S","asm/make_x86_64_sysv_elf_gas.S","asm/ontop_x86_64_sysv_elf_gas.S");
    arch_cc!("unix","arm64" => "asm/jump_arm64_aapcs_elf_gas.S","asm/make_arm64_aapcs_elf_gas.S","asm/ontop_arm64_aapcs_elf_gas.S");
    arch_cc!("unix","arm" => "asm/jump_arm_aapcs_elf_gas.S","asm/make_arm_aapcs_elf_gas.S","asm/ontop_arm_aapcs_elf_gas.S");
    arch_cc!("windows","i386" => "asm/jump_i386_ms_pe_masm.asm","asm/make_i386_ms_pe_masm.asm","asm/ontop_i386_ms_pe_masm.asm");
    arch_cc!("windows","x86_64" => "asm/jump_x86_64_ms_pe_masm.asm","asm/make_x86_64_ms_pe_masm.asm","asm/ontop_x86_64_ms_pe_masm.asm");
}
