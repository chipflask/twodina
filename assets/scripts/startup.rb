class Inventory
  attr_accessor :num_gems

  def initialize
    @num_gems = 0
  end

  def collect_gem(n = 1)
    @num_gems += n
  end
end

@inventory = Inventory.new
