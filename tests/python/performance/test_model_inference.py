"""
Performance tests for model inference in BONGAS-AI
"""

import pytest
import torch
import onnxruntime as ort
import numpy as np
import time
import psutil
import gc
from pathlib import Path
import tempfile
import os

from bongas_ml.models.two_tower import TwoTowerModel
from bongas_ml.export.onnx_exporter import ONNXExporter
from tests.python.fixtures import (
    TestConfig, create_synthetic_user_features, create_synthetic_item_features,
    get_test_device
)


class TestModelInferencePerformance:
    """Performance tests for model inference"""

    def setup_method(self):
        """Setup for each test method"""
        self.temp_dir = tempfile.mkdtemp()
        self.device = get_test_device()
        
        # Create test models
        self.pytorch_model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
            hidden_dims=TestConfig.HIDDEN_DIMS,
        )
        self.pytorch_model.eval()
        
        if self.device == 'cuda':
            self.pytorch_model = self.pytorch_model.cuda()

    def teardown_method(self):
        """Cleanup after each test method"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def measure_inference_time(self, model, user_features, item_features, num_runs=100):
        """Measure inference time for a model"""
        # Warm up
        with torch.no_grad():
            for _ in range(10):
                model(user_features[:1], item_features[:1])
        
        # Measure inference time
        times = []
        with torch.no_grad():
            for _ in range(num_runs):
                start_time = time.perf_counter()
                _ = model(user_features, item_features)
                end_time = time.perf_counter()
                times.append(end_time - start_time)
        
        return np.array(times)

    def measure_onnx_inference_time(self, ort_session, user_features, item_features, num_runs=100):
        """Measure ONNX inference time"""
        # Prepare inputs
        ort_inputs = {
            'user_features': user_features.numpy(),
            'item_features': item_features.numpy(),
        }
        
        # Warm up
        for _ in range(10):
            _ = ort_session.run(None, ort_inputs)
        
        # Measure inference time
        times = []
        for _ in range(num_runs):
            start_time = time.perf_counter()
            _ = ort_session.run(None, ort_inputs)
            end_time = time.perf_counter()
            times.append(end_time - start_time)
        
        return np.array(times)

    def test_pytorch_inference_latency(self):
        """Test PyTorch model inference latency"""
        batch_sizes = [1, 8, 16, 32, 64, 128]
        
        for batch_size in batch_sizes:
            user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
            item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
            
            if self.device == 'cuda':
                user_features = user_features.cuda()
                item_features = item_features.cuda()
            
            times = self.measure_inference_time(
                self.pytorch_model, user_features, item_features, num_runs=50
            )
            
            # Calculate statistics
            mean_time = np.mean(times)
            p95_time = np.percentile(times, 95)
            
            print(f"Batch size {batch_size}: "
                  f"Mean={mean_time*1000:.2f}ms, "
                  f"P95={p95_time*1000:.2f}ms")
            
            # Performance targets (adjust based on hardware)
            if batch_size == 1:
                assert mean_time < 0.1, f"Single inference too slow: {mean_time*1000:.2f}ms"
            elif batch_size == 32:
                assert mean_time < 0.5, f"Batch inference too slow: {mean_time*1000:.2f}ms"

    def test_pytorch_inference_throughput(self):
        """Test PyTorch model inference throughput"""
        batch_sizes = [32, 64, 128, 256]
        
        for batch_size in batch_sizes:
            user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
            item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
            
            if self.device == 'cuda':
                user_features = user_features.cuda()
                item_features = item_features.cuda()
            
            times = self.measure_inference_time(
                self.pytorch_model, user_features, item_features, num_runs=30
            )
            
            # Calculate throughput
            mean_time = np.mean(times)
            throughput = batch_size / mean_time  # predictions per second
            
            print(f"Batch size {batch_size}: "
                  f"Throughput={throughput:.0f} pred/s")
            
            # Performance targets
            if batch_size == 32:
                assert throughput > 1000, f"Throughput too low: {throughput:.0f} pred/s"
            elif batch_size == 128:
                assert throughput > 2000, f"Throughput too low: {throughput:.0f} pred/s"

    def test_onnx_inference_performance(self):
        """Test ONNX model inference performance"""
        # Export model to ONNX
        exporter = ONNXExporter(onnx_dir=self.temp_dir)
        metadata = exporter.export_two_tower(
            model=self.pytorch_model,
            model_name="perf_test_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        # Load ONNX model
        onnx_path = Path(metadata['onnx_path'])
        ort_session = ort.InferenceSession(str(onnx_path))
        
        batch_sizes = [1, 8, 16, 32, 64]
        
        for batch_size in batch_sizes:
            user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
            item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
            
            times = self.measure_onnx_inference_time(
                ort_session, user_features, item_features, num_runs=50
            )
            
            mean_time = np.mean(times)
            p95_time = np.percentile(times, 95)
            
            print(f"ONNX Batch size {batch_size}: "
                  f"Mean={mean_time*1000:.2f}ms, "
                  f"P95={p95_time*1000:.2f}ms")
            
            # ONNX should be reasonably fast
            if batch_size == 1:
                assert mean_time < 0.2, f"ONNX single inference too slow: {mean_time*1000:.2f}ms"
            elif batch_size == 32:
                assert mean_time < 1.0, f"ONNX batch inference too slow: {mean_time*1000:.2f}ms"

    def test_pytorch_vs_onnx_performance_comparison(self):
        """Compare PyTorch vs ONNX performance"""
        # Export model to ONNX
        exporter = ONNXExporter(onnx_dir=self.temp_dir)
        metadata = exporter.export_two_tower(
            model=self.pytorch_model,
            model_name="comparison_model",
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
        )
        
        # Load ONNX model
        onnx_path = Path(metadata['onnx_path'])
        ort_session = ort.InferenceSession(str(onnx_path))
        
        batch_sizes = [1, 16, 64]
        
        for batch_size in batch_sizes:
            user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
            item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
            
            if self.device == 'cuda':
                user_features = user_features.cuda()
                item_features = item_features.cuda()
            
            # Measure PyTorch performance
            pytorch_times = self.measure_inference_time(
                self.pytorch_model, user_features, item_features, num_runs=30
            )
            
            # Measure ONNX performance
            onnx_times = self.measure_onnx_inference_time(
                ort_session, user_features, item_features, num_runs=30
            )
            
            pytorch_mean = np.mean(pytorch_times)
            onnx_mean = np.mean(onnx_times)
            
            speedup = pytorch_mean / onnx_mean if onnx_mean > 0 else float('inf')
            
            print(f"Batch size {batch_size}: "
                  f"PyTorch={pytorch_mean*1000:.2f}ms, "
                  f"ONNX={onnx_mean*1000:.2f}ms, "
                  f"Speedup={speedup:.2f}x")
            
            # ONNX should be competitive with PyTorch
            # Allow some variance due to different implementations
            assert onnx_mean < pytorch_mean * 2.0, \
                f"ONNX too slow compared to PyTorch: {onnx_mean/pytorch_mean:.2f}x"

    def test_memory_usage_during_inference(self):
        """Test memory usage during inference"""
        process = psutil.Process()
        
        # Get baseline memory
        gc.collect()
        baseline_memory = process.memory_info().rss / 1024 / 1024  # MB
        
        # Run inference with different batch sizes
        batch_sizes = [32, 64, 128, 256]
        memory_usage = []
        
        for batch_size in batch_sizes:
            user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
            item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
            
            if self.device == 'cuda':
                user_features = user_features.cuda()
                item_features = item_features.cuda()
            
            # Measure memory during inference
            gc.collect()
            start_memory = process.memory_info().rss / 1024 / 1024
            
            with torch.no_grad():
                for _ in range(10):
                    _ = self.pytorch_model(user_features, item_features)
            
            gc.collect()
            end_memory = process.memory_info().rss / 1024 / 1024
            
            memory_increase = end_memory - start_memory
            memory_usage.append(memory_increase)
            
            print(f"Batch size {batch_size}: Memory increase={memory_increase:.2f}MB")
        
        # Memory usage should scale reasonably with batch size
        # Allow some non-linearity due to caching and other factors
        for i in range(1, len(memory_usage)):
            ratio = memory_usage[i] / memory_usage[0]
            expected_ratio = batch_sizes[i] / batch_sizes[0]
            
            # Memory should not increase more than 3x the expected ratio
            assert ratio < expected_ratio * 3.0, \
                f"Memory usage scaling too fast: batch {batch_sizes[i]} uses {ratio:.2f}x memory of batch {batch_sizes[0]}"

    def test_concurrent_inference_performance(self):
        """Test concurrent inference performance"""
        if self.device != 'cuda':
            pytest.skip("CUDA required for concurrent inference test")
        
        # Create multiple models for concurrent inference
        models = []
        for _ in range(4):
            model = TwoTowerModel(
                user_feature_dim=TestConfig.USER_FEATURE_DIM,
                item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
                embedding_dim=TestConfig.EMBEDDING_DIM,
            )
            model.eval()
            model = model.cuda()
            models.append(model)
        
        # Create test data
        batch_size = 16
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM).cuda()
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM).cuda()
        
        # Measure concurrent inference time
        start_time = time.perf_counter()
        
        # Run inference on all models concurrently
        results = []
        for model in models:
            result = model(user_features, item_features)
            results.append(result)
        
        # Wait for all operations to complete
        torch.cuda.synchronize()
        
        end_time = time.perf_counter()
        concurrent_time = end_time - start_time
        
        # Measure sequential inference time
        start_time = time.perf_counter()
        
        for model in models:
            with torch.no_grad():
                _ = model(user_features, item_features)
        
        torch.cuda.synchronize()
        
        end_time = time.perf_counter()
        sequential_time = end_time - start_time
        
        print(f"Concurrent time: {concurrent_time*1000:.2f}ms")
        print(f"Sequential time: {sequential_time*1000:.2f}ms")
        
        # Concurrent should be faster than sequential
        assert concurrent_time < sequential_time, \
            f"Concurrent inference not faster: {concurrent_time} vs {sequential_time}"

    def test_large_batch_inference(self):
        """Test inference with very large batches"""
        batch_size = 1000
        
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
        
        # Measure inference time
        times = self.measure_inference_time(
            self.pytorch_model, user_features, item_features, num_runs=10
        )
        
        mean_time = np.mean(times)
        throughput = batch_size / mean_time
        
        print(f"Large batch (size {batch_size}): "
              f"Mean={mean_time*1000:.2f}ms, "
              f"Throughput={throughput:.0f} pred/s")
        
        # Should handle large batches reasonably well
        assert mean_time < 0.5, f"Large batch inference too slow: {mean_time*1000:.2f}ms"
        assert throughput > 5000, f"Large batch throughput too low: {throughput:.0f} pred/s"

    def test_model_warmup_effect(self):
        """Test the effect of model warmup on performance"""
        batch_size = 32
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
        
        # Measure cold start time
        start_time = time.perf_counter()
        with torch.no_grad():
            _ = self.pytorch_model(user_features, item_features)
        torch.cuda.synchronize() if self.device == 'cuda' else None
        cold_time = time.perf_counter() - start_time
        
        # Warm up the model
        with torch.no_grad():
            for _ in range(50):
                _ = self.pytorch_model(user_features, item_features)
        
        torch.cuda.synchronize() if self.device == 'cuda' else None
        
        # Measure warm time
        times = self.measure_inference_time(
            self.pytorch_model, user_features, item_features, num_runs=50
        )
        
        warm_mean = np.mean(times)
        
        print(f"Cold start: {cold_time*1000:.2f}ms")
        print(f"Warm mean: {warm_mean*1000:.2f}ms")
        
        # Warm performance should be better
        assert warm_mean < cold_time * 1.5, \
            f"Warm performance not better: {warm_mean} vs {cold_time}"

    def test_different_precision_performance(self):
        """Test performance with different precision (FP16 vs FP32)"""
        if not torch.cuda.is_available():
            pytest.skip("CUDA required for precision comparison")
        
        # Test FP32
        fp32_model = self.pytorch_model.float()
        batch_size = 64
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM).cuda().float()
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM).cuda().float()
        
        fp32_times = self.measure_inference_time(
            fp32_model, user_features, item_features, num_runs=30
        )
        
        # Test FP16
        fp16_model = self.pytorch_model.half()
        user_features_fp16 = user_features.half()
        item_features_fp16 = item_features.half()
        
        fp16_times = self.measure_inference_time(
            fp16_model, user_features_fp16, item_features_fp16, num_runs=30
        )
        
        fp32_mean = np.mean(fp32_times)
        fp16_mean = np.mean(fp16_times)
        
        speedup = fp32_mean / fp16_mean
        
        print(f"FP32: {fp32_mean*1000:.2f}ms")
        print(f"FP16: {fp16_mean*1000:.2f}ms")
        print(f"FP16 speedup: {speedup:.2f}x")
        
        # FP16 should be faster (on compatible hardware)
        # Allow for some variance
        assert fp16_mean < fp32_mean * 1.2, \
            f"FP16 not faster than FP32: {fp16_mean/fp32_mean:.2f}x"


class TestModelInferenceStress:
    """Stress tests for model inference"""

    def setup_method(self):
        """Setup for each test method"""
        self.temp_dir = tempfile.mkdtemp()
        self.device = get_test_device()
        
        self.model = TwoTowerModel(
            user_feature_dim=TestConfig.USER_FEATURE_DIM,
            item_feature_dim=TestConfig.ITEM_FEATURE_DIM,
            embedding_dim=TestConfig.EMBEDDING_DIM,
            hidden_dims=TestConfig.HIDDEN_DIMS,
        )
        self.model.eval()
        
        if self.device == 'cuda':
            self.model = self.model.cuda()

    def teardown_method(self):
        """Cleanup after each test method"""
        import shutil
        shutil.rmtree(self.temp_dir, ignore_errors=True)

    def test_long_duration_inference(self):
        """Test inference performance over long duration"""
        batch_size = 32
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
        
        # Run inference for extended period
        start_time = time.time()
        iteration_times = []
        
        with torch.no_grad():
            while time.time() - start_time < 10.0:  # Run for 10 seconds
                iter_start = time.perf_counter()
                _ = self.model(user_features, item_features)
                iter_end = time.perf_counter()
                iteration_times.append(iter_end - iter_start)
        
        # Analyze performance stability
        times_array = np.array(iteration_times)
        mean_time = np.mean(times_array)
        std_time = np.std(times_array)
        cv = std_time / mean_time  # Coefficient of variation
        
        print(f"Long duration test: "
              f"Mean={mean_time*1000:.2f}ms, "
              f"Std={std_time*1000:.2f}ms, "
              f"CV={cv:.3f}")
        
        # Performance should be stable (low coefficient of variation)
        assert cv < 0.1, f"Performance too unstable: CV={cv:.3f}"

    def test_memory_leak_detection(self):
        """Test for memory leaks during repeated inference"""
        process = psutil.Process()
        
        # Get initial memory
        gc.collect()
        initial_memory = process.memory_info().rss / 1024 / 1024
        
        batch_size = 64
        user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
        item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
        
        if self.device == 'cuda':
            user_features = user_features.cuda()
            item_features = item_features.cuda()
        
        # Run many inference iterations
        with torch.no_grad():
            for i in range(1000):
                _ = self.model(user_features, item_features)
                
                # Check memory every 100 iterations
                if i % 100 == 0:
                    gc.collect()
                    current_memory = process.memory_info().rss / 1024 / 1024
                    memory_increase = current_memory - initial_memory
                    
                    print(f"Iteration {i}: Memory increase={memory_increase:.2f}MB")
                    
                    # Memory should not grow excessively
                    assert memory_increase < 100.0, \
                        f"Possible memory leak detected: {memory_increase:.2f}MB increase"

    def test_extreme_batch_sizes(self):
        """Test inference with extreme batch sizes"""
        extreme_batch_sizes = [1, 1000, 5000]
        
        for batch_size in extreme_batch_sizes:
            try:
                user_features = torch.randn(batch_size, TestConfig.USER_FEATURE_DIM)
                item_features = torch.randn(batch_size, TestConfig.ITEM_FEATURE_DIM)
                
                if self.device == 'cuda':
                    user_features = user_features.cuda()
                    item_features = item_features.cuda()
                
                # Measure inference time
                start_time = time.perf_counter()
                with torch.no_grad():
                    scores = self.model(user_features, item_features)
                end_time = time.perf_counter()
                
                inference_time = end_time - start_time
                throughput = batch_size / inference_time
                
                print(f"Extreme batch size {batch_size}: "
                      f"Time={inference_time*1000:.2f}ms, "
                      f"Throughput={throughput:.0f} pred/s")
                
                # Should complete without errors
                assert scores.shape == (batch_size,)
                assert torch.isfinite(scores).all()
                
                # Should complete in reasonable time
                assert inference_time < 5.0, \
                    f"Extreme batch inference too slow: {inference_time:.2f}s"
                
            except RuntimeError as e:
                if "out of memory" in str(e).lower():
                    print(f"OOM for batch size {batch_size}, skipping")
                    continue
                else:
                    raise


if __name__ == "__main__":
    pytest.main([__file__, "-v"])