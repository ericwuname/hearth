// Calculator Logic Test Suite
// Tests the core calculator logic extracted from calc.html

let currentInput = '0';
let shouldResetDisplay = false;

function updateDisplay() {
    // Mock - would update DOM in real app
}

function appendNumber(num) {
    if (currentInput === '0' || shouldResetDisplay) {
        currentInput = num;
        shouldResetDisplay = false;
    } else {
        currentInput += num;
    }
    updateDisplay();
}

function appendOperator(op) {
    const lastChar = currentInput.slice(-1);
    if (['+', '-', '*', '/'].includes(lastChar)) {
        currentInput = currentInput.slice(0, -1) + op;
    } else {
        currentInput += op;
    }
    shouldResetDisplay = false;
    updateDisplay();
}

function clearDisplay() {
    currentInput = '0';
    shouldResetDisplay = false;
    updateDisplay();
}

function calculate() {
    try {
        const result = new Function('return ' + currentInput)();
        const roundedResult = Math.round(result * 1000000000) / 1000000000;
        currentInput = roundedResult.toString();
        shouldResetDisplay = true;
        updateDisplay();
    } catch (error) {
        currentInput = '错误';
        shouldResetDisplay = true;
        updateDisplay();
    }
}

// Test cases
const tests = [
    { input: ['2', '+', '2'], expected: '4', desc: '加法: 2+2' },
    { input: ['5', '-', '3'], expected: '2', desc: '减法: 5-3' },
    { input: ['4', '*', '3'], expected: '12', desc: '乘法: 4×3' },
    { input: ['10', '/', '2'], expected: '5', desc: '除法: 10÷2' },
    { input: ['15', '+', '20'], expected: '35', desc: '两位数加法: 15+20' },
    { input: ['2.5', '*', '4'], expected: '10', desc: '小数乘法: 2.5×4' },
    { input: ['100', '/', '3'], expected: '33.33333333', desc: '除不尽: 100÷3（精度测试）' },
    { input: ['0'], expected: '0', desc: '初始值: 0' },
    { input: ['7'], expected: '7', desc: '单个数字: 7' },
    { input: ['1', '2', '+', '3', '4'], expected: '46', desc: '复合运算: 12+34' },
];

console.log('=== 计算器逻辑自测 ===\n');

let passCount = 0;
tests.forEach((test, i) => {
    clearDisplay();
    test.input.forEach(item => {
        if (['+', '-', '*', '/'].includes(item)) {
            appendOperator(item);
        } else {
            appendNumber(item);
        }
    });
    calculate();
    
    const passed = currentInput === test.expected;
    const status = passed ? '✓ PASS' : '✗ FAIL';
    console.log(`${status} | 测试${i + 1}: ${test.desc}`);
    console.log(`       输入: [${test.input.join(', ')}]`);
    console.log(`       期望: ${test.expected}, 实际: ${currentInput}\n`);
    
    if (passed) passCount++;
});

console.log(`=== 测试结果: ${passCount}/${tests.length} 通过 ===\n`);

if (passCount === tests.length) {
    console.log('✓ 所有测试通过！计算器逻辑验证成功。');
    process.exit(0);
} else {
    console.log('✗ 部分测试失败，需要修复逻辑。');
    process.exit(1);
}
