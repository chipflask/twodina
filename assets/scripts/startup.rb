require_relative 'prelude'

class Game < BaseGame
  attr_accessor :shared_score

  def initialize
    @shared_score = 0
  end

end

# class Player < BasePlayer
#   attr_accessor :num_gems

#   def initialize(**kwargs)
#     super(**kwargs)
#     @num_gems = 0
#   end
# end

player do
  define :num_gems

  setup do
    @num_gems = 0
  end

  on_collect :gem do
    @num_gems += 1
  end
  # on_collect(:big_gem) { @num_gems += 5 }
  on_collect :any do
    total = players.map {|pl| pl.num_gems }.sum
    puts "total gems: #{total}"
  end
end

# This is optional, in case you want to use custom parameters for the
# constructor.
# game = Game.new
