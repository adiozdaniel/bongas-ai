#!/usr/bin/env python3
"""
Test runner script for BONGAS-AI Python tests

This script provides a convenient way to run different test suites
with appropriate configurations and reporting.
"""

import os
import sys
import subprocess
import argparse
import shutil
from pathlib import Path


def setup_environment():
    """Setup the test environment"""
    # Add the project root to Python path
    project_root = Path(__file__).parent.parent.parent
    sys.path.insert(0, str(project_root))
    
    # Create reports directory
    reports_dir = Path(__file__).parent / "reports"
    reports_dir.mkdir(exist_ok=True)
    
    return project_root, reports_dir


def install_requirements():
    """Install test requirements"""
    print("Installing test requirements...")
    requirements_file = Path(__file__).parent / "requirements.txt"
    
    if requirements_file.exists():
        try:
            subprocess.run([
                sys.executable, "-m", "pip", "install", "-r", str(requirements_file)
            ], check=True)
            print("✓ Requirements installed successfully")
        except subprocess.CalledProcessError as e:
            print(f"✗ Failed to install requirements: {e}")
            return False
    else:
        print("✗ Requirements file not found")
        return False
    
    return True


def run_unit_tests(verbose=True):
    """Run unit tests"""
    print("\n" + "="*60)
    print("RUNNING UNIT TESTS")
    print("="*60)
    
    cmd = [
        sys.executable, "-m", "pytest",
        "tests/python/unit/",
        "-m", "unit",
        "--html=tests/python/reports/unit_test_report.html",
        "--json-report",
        "--json-report-file=tests/python/reports/unit_test_report.json"
    ]
    
    if verbose:
        cmd.append("-v")
    
    result = subprocess.run(cmd, cwd=os.path.dirname(__file__))
    return result.returncode == 0


def run_integration_tests(verbose=True):
    """Run integration tests"""
    print("\n" + "="*60)
    print("RUNNING INTEGRATION TESTS")
    print("="*60)
    
    cmd = [
        sys.executable, "-m", "pytest",
        "tests/python/integration/",
        "-m", "integration",
        "--html=tests/python/reports/integration_test_report.html",
        "--json-report",
        "--json-report-file=tests/python/reports/integration_test_report.json"
    ]
    
    if verbose:
        cmd.append("-v")
    
    result = subprocess.run(cmd, cwd=os.path.dirname(__file__))
    return result.returncode == 0


def run_performance_tests(verbose=True):
    """Run performance tests"""
    print("\n" + "="*60)
    print("RUNNING PERFORMANCE TESTS")
    print("="*60)
    
    cmd = [
        sys.executable, "-m", "pytest",
        "tests/python/performance/",
        "-m", "performance",
        "--html=tests/python/reports/performance_test_report.html",
        "--json-report",
        "--json-report-file=tests/python/reports/performance_test_report.json"
    ]
    
    if verbose:
        cmd.append("-v")
    
    result = subprocess.run(cmd, cwd=os.path.dirname(__file__))
    return result.returncode == 0


def run_all_tests(verbose=True):
    """Run all tests"""
    print("\n" + "="*60)
    print("RUNNING ALL TESTS")
    print("="*60)
    
    cmd = [
        sys.executable, "-m", "pytest",
        "tests/python/",
        "--html=tests/python/reports/all_test_report.html",
        "--json-report",
        "--json-report-file=tests/python/reports/all_test_report.json"
    ]
    
    if verbose:
        cmd.append("-v")
    
    result = subprocess.run(cmd, cwd=os.path.dirname(__file__))
    return result.returncode == 0


def run_coverage_report():
    """Generate coverage report"""
    print("\n" + "="*60)
    print("GENERATING COVERAGE REPORT")
    print("="*60)
    
    cmd = [
        sys.executable, "-m", "pytest",
        "tests/python/",
        "--cov=bongas_ml",
        "--cov-report=html:tests/python/coverage_html",
        "--cov-report=term-missing",
        "--cov-report=xml:tests/python/reports/coverage.xml"
    ]
    
    result = subprocess.run(cmd, cwd=os.path.dirname(__file__))
    return result.returncode == 0


def run_specific_test_file(test_file, verbose=True):
    """Run a specific test file"""
    print(f"\n" + "="*60)
    print(f"RUNNING SPECIFIC TEST: {test_file}")
    print("="*60)
    
    cmd = [
        sys.executable, "-m", "pytest",
        test_file,
        "--html=tests/python/reports/specific_test_report.html",
        "--json-report",
        "--json-report-file=tests/python/reports/specific_test_report.json"
    ]
    
    if verbose:
        cmd.append("-v")
    
    result = subprocess.run(cmd, cwd=os.path.dirname(__file__))
    return result.returncode == 0


def clean_reports():
    """Clean test reports and coverage data"""
    print("Cleaning test reports and coverage data...")
    
    reports_dir = Path(__file__).parent / "reports"
    coverage_dir = Path(__file__).parent / "coverage_html"
    
    if reports_dir.exists():
        shutil.rmtree(reports_dir)
        print("✓ Reports directory cleaned")
    
    if coverage_dir.exists():
        shutil.rmtree(coverage_dir)
        print("✓ Coverage directory cleaned")
    
    # Remove coverage files
    for pattern in ["*.coverage", ".coverage", "coverage.xml"]:
        for file in Path(__file__).parent.glob(pattern):
            file.unlink()
    
    print("✓ Cleanup completed")


def print_test_summary():
    """Print test summary and instructions"""
    print("\n" + "="*60)
    print("BONGAS-AI PYTHON TESTS")
    print("="*60)
    print("\nAvailable test suites:")
    print("  • Unit tests: Core functionality tests")
    print("  • Integration tests: End-to-end workflow tests")
    print("  • Performance tests: Speed and memory tests")
    print("\nUsage examples:")
    print("  python run_tests.py --unit          # Run unit tests")
    print("  python run_tests.py --integration   # Run integration tests")
    print("  python run_tests.py --performance   # Run performance tests")
    print("  python run_tests.py --all           # Run all tests")
    print("  python run_tests.py --coverage      # Generate coverage report")
    print("  python run_tests.py --file <file>   # Run specific test file")
    print("  python run_tests.py --clean         # Clean reports")
    print("\nReports will be generated in: tests/python/reports/")
    print("Coverage reports in: tests/python/coverage_html/")
    print("="*60)


def main():
    """Main test runner function"""
    parser = argparse.ArgumentParser(description="BONGAS-AI Python Test Runner")
    
    parser.add_argument("--unit", action="store_true", help="Run unit tests")
    parser.add_argument("--integration", action="store_true", help="Run integration tests")
    parser.add_argument("--performance", action="store_true", help="Run performance tests")
    parser.add_argument("--all", action="store_true", help="Run all tests")
    parser.add_argument("--coverage", action="store_true", help="Generate coverage report")
    parser.add_argument("--file", type=str, help="Run specific test file")
    parser.add_argument("--verbose", "-v", action="store_true", help="Verbose output")
    parser.add_argument("--install", action="store_true", help="Install requirements")
    parser.add_argument("--clean", action="store_true", help="Clean reports")
    parser.add_argument("--summary", action="store_true", help="Show test summary")
    
    args = parser.parse_args()
    
    # Setup environment
    project_root, reports_dir = setup_environment()
    
    # Show summary if requested
    if args.summary:
        print_test_summary()
        return
    
    # Clean reports if requested
    if args.clean:
        clean_reports()
        return
    
    # Install requirements if requested
    if args.install:
        if not install_requirements():
            sys.exit(1)
    
    # Run tests based on arguments
    success = True
    
    if args.unit:
        success &= run_unit_tests(args.verbose)
    elif args.integration:
        success &= run_integration_tests(args.verbose)
    elif args.performance:
        success &= run_performance_tests(args.verbose)
    elif args.all:
        success &= run_all_tests(args.verbose)
    elif args.coverage:
        success &= run_coverage_report()
    elif args.file:
        success &= run_specific_test_file(args.file, args.verbose)
    else:
        # Default: show summary
        print_test_summary()
        return
    
    # Print final result
    if success:
        print("\n" + "="*60)
        print("✓ ALL TESTS PASSED!")
        print("="*60)
        sys.exit(0)
    else:
        print("\n" + "="*60)
        print("✗ SOME TESTS FAILED!")
        print("="*60)
        sys.exit(1)


if __name__ == "__main__":
    main()