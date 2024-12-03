# build.ps1

# Function to check and set required environment variables
function Set-RequiredEnvironment {
    # Ensure USERPROFILE is set
    if (-not $env:USERPROFILE) {
        Write-Error "USERPROFILE environment variable not found"
        exit 1
    }

    # Set required environment variables
    $env:HOME = $env:USERPROFILE
    $env:RUST_BACKTRACE = "full"
    
    # Set focused logging for better error visibility
    $env:RUST_LOG = "solana_runtime::system_instruction_processor=info,solana_runtime::message_processor=info,solana_bpf_loader=info,solana_rbpf=info"

    # Add Solana to PATH if not already present
    $solanaPath = Join-Path $env:USERPROFILE ".local\share\solana\install\active_release\bin"
    if ($env:PATH -notlike "*$solanaPath*") {
        $env:PATH = "$solanaPath;$env:PATH"
    }

    # Verify critical paths exist
    $configDir = Join-Path $env:USERPROFILE ".config\solana"
    if (-not (Test-Path $configDir)) {
        New-Item -ItemType Directory -Force -Path $configDir | Out-Null
        Write-Host "Created Solana config directory at $configDir"
    }
}

# Function to verify prerequisites
function Test-Prerequisites {
    # Check Rust installation
    if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
        Write-Error "Rust is not installed. Please install from https://rustup.rs/"
        exit 1
    }

    # Check Solana CLI
    if (-not (Get-Command solana -ErrorAction SilentlyContinue)) {
        Write-Error "Solana CLI is not installed or not in PATH"
        exit 1
    }

    # Display versions for debugging
    Write-Host "Rust version: $(rustc --version)"
    Write-Host "Solana version: $(solana --version)"
}

# Main execution
try {
    Write-Host "`nSetting up environment..."
    Set-RequiredEnvironment

    Write-Host "`nVerifying prerequisites..."
    Test-Prerequisites

    Write-Host "`nBuilding Mr. Fresh program..."
    cargo build-sbf --verbose

    if ($LASTEXITCODE -eq 0) {
        Write-Host "`nBuild successful! Attempting deployment to devnet..."
        $programKeypath = Join-Path $env:USERPROFILE ".config\solana\mr_fresh-keypair.json"
        
        if (-not (Test-Path $programKeypath)) {
            Write-Host "Generating program keypair..."
            solana-keygen new -o $programKeypath --no-bip39-passphrase
        }
        
        $buildDir = "target\deploy"
        if (Test-Path "$buildDir\mr_fresh.so") {
            solana program deploy "$buildDir\mr_fresh.so" --program-id $programKeypath
        } else {
            Write-Error "Built program not found at $buildDir\mr_fresh.so"
            exit 1
        }
    } else {
        Write-Error "Build failed. See error messages above."
        exit 1
    }
} catch {
    Write-Error "An error occurred: $_"
    exit 1
}