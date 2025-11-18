import cv2 as cv
import numpy as np
from pathlib import Path

# 图像列表，与 Rust 测试保持一致
image_names = ["in_battle", "1-4_deploying", "1-4_deploying_direction"]

# 模板列表，与 Rust 测试保持一致
template_names = ["battle_pause", "battle_deploy-card-cost1"]

# 方法映射：OpenCV 方法名 -> 统一的方法名字符串（与 Rust Display 实现保持一致）
method_mapping = {
    'TM_SQDIFF': 'sqdiff',
    'TM_SQDIFF_NORMED': 'sqdiff_normed',
    'TM_CCORR': 'ccorr',
    'TM_CCORR_NORMED': 'ccorr_normed',
    'TM_CCOEFF': 'ccoeff',
    'TM_CCOEFF_NORMED': 'ccoeff_normed',
}

# 需要归一化的方法（与 Rust 版本保持一致）
methods_need_normalize = [cv.TM_SQDIFF, cv.TM_CCORR, cv.TM_CCOEFF]

# 所有6种方法
methods = ['TM_SQDIFF', 'TM_SQDIFF_NORMED', 'TM_CCORR',
           'TM_CCORR_NORMED', 'TM_CCOEFF', 'TM_CCOEFF_NORMED']

# 处理每个模板
for template_name in template_names:
    template_path = f"../assets/{template_name}.png"
    template_output_dir = Path(f"../assets/output/{template_name}")
    
    # 加载模板
    template = cv.imread(template_path, cv.IMREAD_GRAYSCALE)
    assert template is not None, f"模板文件无法读取: {template_path}"
    
    print(f"\n处理模板: {template_name}")
    
    for meth in methods:
        method = getattr(cv, meth)
        method_name = method_mapping[meth]
        
        # 为每个方法创建子目录
        method_dir = template_output_dir / method_name
        method_dir.mkdir(parents=True, exist_ok=True)
        
        for image_name in image_names:
            # 加载图像
            image_path = f"../assets/{image_name}.png"
            img = cv.imread(image_path, cv.IMREAD_GRAYSCALE)
            assert img is not None, f"图像文件无法读取: {image_path}"
            
            print(f"  使用 {meth} 匹配 {image_name}...")
            
            # 应用模板匹配
            res = cv.matchTemplate(img, template, method)
            
            # 对于需要归一化的方法，进行线性 min-max 归一化
            if method in methods_need_normalize:
                res_min = res.min()
                res_max = res.max()
                if res_max > res_min:
                    res = (res - res_min) / (res_max - res_min)
                else:
                    res = np.zeros_like(res)
            
            # 将结果转换为 0-255 范围的 uint8
            res_uint8 = (res * 255.0).astype(np.uint8)
            
            # 保存结果
            output_path = method_dir / f"{image_name}-opencv.png"
            cv.imwrite(str(output_path), res_uint8)
            print(f"    结果已保存到: {output_path}")

print("\n所有结果已生成完成！")